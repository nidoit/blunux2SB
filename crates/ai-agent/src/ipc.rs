use serde::{Deserialize, Serialize};

/// IPC message types for Phase 2 WhatsApp bridge communication.
/// These types are defined now but the runtime (Unix socket listener)
/// is not implemented until Phase 2.

pub fn socket_path() -> std::path::PathBuf {
    let uid = std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "1000".into());
    std::path::PathBuf::from(format!("/run/user/{uid}/blunux-ai.sock"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    #[serde(rename = "type")]
    pub msg_type: IpcMessageType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IpcMessageType {
    Message,
    Response,
    Action,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_message_serde_roundtrip() {
        let msg = IpcMessage {
            msg_type: IpcMessageType::Message,
            from: Some("+821012345678".into()),
            body: Some("안녕하세요".into()),
            to: None,
            actions: None,
            action: None,
            timestamp: Some("2026-02-20T09:00:00Z".into()),
        };

        let json = serde_json::to_string(&msg).unwrap();
        let parsed: IpcMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.msg_type, IpcMessageType::Message);
        assert_eq!(parsed.from.as_deref(), Some("+821012345678"));
        assert_eq!(parsed.body.as_deref(), Some("안녕하세요"));
    }

    #[test]
    fn test_ipc_response_message() {
        let msg = IpcMessage {
            msg_type: IpcMessageType::Response,
            from: None,
            body: Some("System updated successfully".into()),
            to: Some("+821012345678".into()),
            actions: Some(vec!["OK".into(), "Show logs".into()]),
            action: None,
            timestamp: None,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"response\""));
        assert!(json.contains("actions"));
    }
}
