pub mod packages;
pub mod safety;
pub mod services;
pub mod system;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::ToolError;
pub use safety::{PermissionLevel, SafetyChecker, SafetyResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[async_trait]
pub trait SystemTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn input_schema(&self) -> serde_json::Value;
    fn permission_level(&self) -> PermissionLevel;
    async fn execute(&self, input: serde_json::Value) -> Result<String, ToolError>;

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            input_schema: self.input_schema(),
        }
    }
}

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn SystemTool>>,
}

impl ToolRegistry {
    pub fn default_tools() -> Self {
        let mut tools: HashMap<String, Box<dyn SystemTool>> = HashMap::new();

        // System tools
        tools.insert("check_disk".into(), Box::new(system::CheckDiskTool));
        tools.insert("check_memory".into(), Box::new(system::CheckMemoryTool));
        tools.insert(
            "check_processes".into(),
            Box::new(system::CheckProcessesTool),
        );
        tools.insert("read_logs".into(), Box::new(system::ReadLogsTool));
        tools.insert(
            "check_network".into(),
            Box::new(system::CheckNetworkTool),
        );

        // Package tools
        tools.insert(
            "list_packages".into(),
            Box::new(packages::ListPackagesTool),
        );
        tools.insert(
            "install_package".into(),
            Box::new(packages::InstallPackageTool),
        );
        tools.insert(
            "remove_package".into(),
            Box::new(packages::RemovePackageTool),
        );
        tools.insert(
            "update_system".into(),
            Box::new(packages::UpdateSystemTool),
        );

        // Service tools
        tools.insert(
            "manage_service".into(),
            Box::new(services::ManageServiceTool),
        );

        // Generic command
        tools.insert("run_command".into(), Box::new(system::RunCommandTool));

        Self { tools }
    }

    pub fn get(&self, name: &str) -> Option<&dyn SystemTool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }
}
