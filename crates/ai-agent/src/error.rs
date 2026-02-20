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
    Parse(String),

    #[error("Empty response from provider")]
    EmptyResponse,
}

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

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Failed to read memory file {path}: {source}")]
    Read { path: String, source: std::io::Error },

    #[error("Failed to write memory file {path}: {source}")]
    Write { path: String, source: std::io::Error },
}

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

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_provider() {
        let err = ProviderError::ApiError {
            status: 401,
            message: "unauthorized".into(),
        };
        assert!(format!("{err}").contains("401"));
        assert!(format!("{err}").contains("unauthorized"));
    }

    #[test]
    fn test_error_display_tool() {
        let err = ToolError::ExecutionFailed {
            command: "df -h".into(),
            exit_code: 1,
            stderr: "not found".into(),
        };
        assert!(format!("{err}").contains("df -h"));
    }

    #[test]
    fn test_error_display_config() {
        let err = ConfigError::NotFound {
            path: "/etc/config.toml".into(),
        };
        assert!(format!("{err}").contains("/etc/config.toml"));
    }

    #[test]
    fn test_error_display_memory() {
        let err = MemoryError::Read {
            path: "SYSTEM.md".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        };
        assert!(format!("{err}").contains("SYSTEM.md"));
    }

    #[test]
    fn test_agent_error_from_provider() {
        let pe = ProviderError::AuthenticationFailed;
        let ae: AgentError = pe.into();
        assert!(format!("{ae}").contains("Authentication failed"));
    }
}
