use async_trait::async_trait;
use std::time::Duration;
use tokio::process::Command;

use crate::error::ToolError;
use crate::tools::{PermissionLevel, SystemTool};

async fn run_pkg_cmd(cmd: &str, args: &[&str], timeout_secs: u64) -> Result<String, ToolError> {
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
        Err(ToolError::ExecutionFailed {
            command: format!("{cmd} {}", args.join(" ")),
            exit_code: result.status.code().unwrap_or(-1),
            stderr,
        })
    }
}

// ── list_packages ────────────────────────────────────────────────────────────

pub struct ListPackagesTool;

#[async_trait]
impl SystemTool for ListPackagesTool {
    fn name(&self) -> &str {
        "list_packages"
    }
    fn description(&self) -> &str {
        "List installed packages. Optionally search for a specific package."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "search": {
                    "type": "string",
                    "description": "Optional search query to filter packages"
                }
            },
            "required": []
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Safe
    }
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError> {
        if let Some(query) = input.get("search").and_then(|v| v.as_str()) {
            run_pkg_cmd("pacman", &["-Qs", query], 60).await
        } else {
            run_pkg_cmd("pacman", &["-Q"], 60).await
        }
    }
}

// ── install_package ──────────────────────────────────────────────────────────

pub struct InstallPackageTool;

#[async_trait]
impl SystemTool for InstallPackageTool {
    fn name(&self) -> &str {
        "install_package"
    }
    fn description(&self) -> &str {
        "Install a package using yay (supports AUR and official repos)."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "package": {
                    "type": "string",
                    "description": "Package name to install (e.g. 'google-chrome', 'vlc')"
                }
            },
            "required": ["package"]
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::RequiresConfirmation
    }
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError> {
        let package = input
            .get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'package' field".into()))?;

        // Validate package name (alphanumeric, dash, underscore, dot only)
        if !package
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(ToolError::InvalidInput(format!(
                "Invalid package name: {package}"
            )));
        }

        run_pkg_cmd("yay", &["-S", "--noconfirm", package], 300).await
    }
}

// ── remove_package ───────────────────────────────────────────────────────────

pub struct RemovePackageTool;

#[async_trait]
impl SystemTool for RemovePackageTool {
    fn name(&self) -> &str {
        "remove_package"
    }
    fn description(&self) -> &str {
        "Remove a package and its unneeded dependencies."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "package": {
                    "type": "string",
                    "description": "Package name to remove"
                }
            },
            "required": ["package"]
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::RequiresConfirmation
    }
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError> {
        let package = input
            .get("package")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidInput("Missing 'package' field".into()))?;

        if !package
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(ToolError::InvalidInput(format!(
                "Invalid package name: {package}"
            )));
        }

        run_pkg_cmd("yay", &["-Rns", "--noconfirm", package], 120).await
    }
}

// ── update_system ────────────────────────────────────────────────────────────

pub struct UpdateSystemTool;

#[async_trait]
impl SystemTool for UpdateSystemTool {
    fn name(&self) -> &str {
        "update_system"
    }
    fn description(&self) -> &str {
        "Perform a full system update (pacman -Syu). This updates all installed packages to their latest versions."
    }
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::RequiresConfirmation
    }
    async fn execute(&self, _input: serde_json::Value) -> Result<String, ToolError> {
        run_pkg_cmd("sudo", &["pacman", "-Syu", "--noconfirm"], 600).await
    }
}
