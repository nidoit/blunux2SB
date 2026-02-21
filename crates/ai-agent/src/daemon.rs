use std::collections::VecDeque;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;

use crate::agent::Agent;
use crate::automations::{run_scheduler, AutomationsConfig};
use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::ipc::{socket_path, IpcMessage, IpcMessageType};

/// Pending outbound notifications queued by the automation scheduler.
/// Each entry is `(phone_number, message_body)`.
type NotifyQueue = Arc<Mutex<VecDeque<(String, String)>>>;

/// Run the AI agent daemon, listening on a Unix domain socket.
///
/// Incoming messages are newline-delimited JSON `IpcMessage` objects.
/// For each `Message` type, the agent processes the request and writes
/// a `Response` message back on the same connection.
///
/// A background scheduler task fires automations on their cron schedules and
/// pushes results to `notify_queue`.  The WhatsApp bridge polls the queue via
/// the `poll_notifications` IPC action.
pub async fn run_daemon(config: &AgentConfig) -> Result<(), AgentError> {
    let path = socket_path();

    // Remove stale socket file if present
    if path.exists() {
        std::fs::remove_file(&path).map_err(AgentError::Io)?;
    }

    let listener = UnixListener::bind(&path).map_err(AgentError::Io)?;

    // Set socket permissions so only the current user can connect
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(AgentError::Io)?;
    }

    eprintln!("[blunux-ai daemon] Listening on {}", path.display());

    // Write default automations.toml if not present
    let _ = AutomationsConfig::write_defaults(&config.config_dir);

    let agent = Arc::new(Mutex::new(Agent::new_daemon(config)?));
    let notify_queue: NotifyQueue = Arc::new(Mutex::new(VecDeque::new()));

    // Spawn automation scheduler as a background task
    let sched_agent = Arc::clone(&agent);
    let sched_queue = Arc::clone(&notify_queue);
    let sched_wa_cfg = config.whatsapp.clone();
    let sched_config_dir = config.config_dir.clone();
    tokio::spawn(async move {
        run_scheduler(sched_agent, sched_queue, sched_wa_cfg, sched_config_dir).await;
    });

    loop {
        let (stream, _addr) = listener.accept().await.map_err(AgentError::Io)?;
        let agent = Arc::clone(&agent);
        let queue = Arc::clone(&notify_queue);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, agent, queue).await {
                eprintln!("[blunux-ai daemon] connection error: {e}");
            }
        });
    }
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    agent: Arc<Mutex<Agent>>,
    notify_queue: NotifyQueue,
) -> Result<(), AgentError> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await.map_err(AgentError::Io)? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let msg: IpcMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                let err_resp = error_response(None, &format!("Invalid JSON: {e}"));
                let mut json = serde_json::to_string(&err_resp).unwrap_or_default();
                json.push('\n');
                let _ = writer.write_all(json.as_bytes()).await;
                continue;
            }
        };

        let response = process_ipc_message(msg, &agent, &notify_queue).await;
        let mut json = serde_json::to_string(&response).unwrap_or_default();
        json.push('\n');
        writer.write_all(json.as_bytes()).await.map_err(AgentError::Io)?;
    }

    Ok(())
}

async fn process_ipc_message(
    msg: IpcMessage,
    agent: &Arc<Mutex<Agent>>,
    notify_queue: &NotifyQueue,
) -> IpcMessage {
    match msg.msg_type {
        IpcMessageType::Message => {
            let phone = match &msg.from {
                Some(p) => p.clone(),
                None => {
                    return error_response(None, "Missing 'from' field");
                }
            };
            let body = match &msg.body {
                Some(b) => b.clone(),
                None => {
                    return error_response(Some(&phone), "Missing 'body' field");
                }
            };

            let mut locked = agent.lock().await;
            match locked.chat_as_user(&phone, &body).await {
                Ok(reply) => IpcMessage {
                    msg_type: IpcMessageType::Response,
                    from: None,
                    body: Some(reply),
                    to: Some(phone),
                    actions: None,
                    action: None,
                    notifications: None,
                    timestamp: Some(utc_now()),
                },
                Err(e) => error_response(Some(&phone), &e.to_string()),
            }
        }
        IpcMessageType::Action => {
            let action = msg.action.as_deref().unwrap_or("");
            match action {
                "ping" => IpcMessage {
                    msg_type: IpcMessageType::Response,
                    from: None,
                    body: Some("pong".into()),
                    to: msg.from.clone(),
                    actions: None,
                    action: None,
                    notifications: None,
                    timestamp: Some(utc_now()),
                },
                "reset" => {
                    let phone = msg.from.as_deref().unwrap_or("");
                    if !phone.is_empty() {
                        let mut locked = agent.lock().await;
                        locked.reset_user_conversation(phone);
                    }
                    IpcMessage {
                        msg_type: IpcMessageType::Response,
                        from: None,
                        body: Some("Conversation reset.".into()),
                        to: msg.from.clone(),
                        actions: None,
                        action: None,
                        notifications: None,
                        timestamp: Some(utc_now()),
                    }
                }
                "poll_notifications" => {
                    // Drain up to 10 pending notifications per poll to avoid
                    // sending a huge payload in one response.
                    let mut queue = notify_queue.lock().await;
                    let take = queue.len().min(10);
                    let batch: Vec<(String, String)> = queue.drain(..take).collect();
                    drop(queue);

                    let items: Vec<serde_json::Value> = batch
                        .into_iter()
                        .map(|(to, body)| {
                            serde_json::json!({ "to": to, "body": body })
                        })
                        .collect();

                    IpcMessage {
                        msg_type: IpcMessageType::Response,
                        from: None,
                        body: None,
                        to: msg.from.clone(),
                        actions: None,
                        action: None,
                        notifications: Some(items),
                        timestamp: Some(utc_now()),
                    }
                }
                other => error_response(msg.from.as_deref(), &format!("Unknown action: {other}")),
            }
        }
        IpcMessageType::Response => {
            error_response(None, "Unexpected message type 'response' from client")
        }
    }
}

fn error_response(to: Option<&str>, reason: &str) -> IpcMessage {
    IpcMessage {
        msg_type: IpcMessageType::Response,
        from: None,
        body: Some(format!("Error: {reason}")),
        to: to.map(|s| s.to_string()),
        actions: None,
        action: None,
        notifications: None,
        timestamp: Some(utc_now()),
    }
}

fn utc_now() -> String {
    chrono::Utc::now().to_rfc3339()
}
