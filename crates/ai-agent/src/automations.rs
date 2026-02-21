use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Datelike, Local, Timelike};
use tokio::sync::Mutex;

use crate::agent::Agent;
use crate::config::WhatsAppConfig;

// ─── Automation config ────────────────────────────────────────────────────────

/// A single scheduled automation entry from automations.toml.
#[derive(Debug, Clone)]
pub struct Automation {
    /// Human-readable label.
    pub name: String,
    /// 5-field cron expression: "min hour dom month dow"
    pub schedule: String,
    /// Natural-language action sent to the AI agent.
    pub action: String,
    /// Notification channel — currently only "whatsapp" is supported.
    pub notify: String,
    /// When true, the agent is allowed to execute safe actions without
    /// asking for confirmation (already the default in daemon mode).
    pub auto_apply: bool,
    /// Master on/off switch. Defaults to true.
    pub enabled: bool,
}

/// All automations loaded from `~/.config/blunux-ai/automations.toml`.
#[derive(Debug, Default)]
pub struct AutomationsConfig {
    pub automations: Vec<Automation>,
}

impl AutomationsConfig {
    /// Load automations from `<config_dir>/automations.toml`.
    /// Missing file → empty config (no automations scheduled).
    pub fn load(config_dir: &Path) -> Self {
        let path = config_dir.join("automations.toml");
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let table: toml::Table = match toml::from_str(&content) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[automations] Parse error: {e}");
                return Self::default();
            }
        };

        let entries = match table.get("automation").and_then(|v| v.as_array()) {
            Some(arr) => arr,
            None => return Self::default(),
        };

        let mut automations = Vec::new();
        for entry in entries {
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed")
                .to_string();
            let schedule = entry
                .get("schedule")
                .and_then(|v| v.as_str())
                .unwrap_or("0 9 * * *")
                .to_string();
            let action = match entry.get("action").and_then(|v| v.as_str()) {
                Some(a) => a.to_string(),
                None => {
                    eprintln!("[automations] Skipping '{name}': missing 'action' field");
                    continue;
                }
            };
            let notify = entry
                .get("notify")
                .and_then(|v| v.as_str())
                .unwrap_or("whatsapp")
                .to_string();
            let auto_apply = entry
                .get("auto_apply")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let enabled = entry
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            automations.push(Automation {
                name,
                schedule,
                action,
                notify,
                auto_apply,
                enabled,
            });
        }

        Self { automations }
    }

    /// Write the default automations.toml template if none exists yet.
    pub fn write_defaults(config_dir: &Path) -> std::io::Result<()> {
        let path = config_dir.join("automations.toml");
        if path.exists() {
            return Ok(());
        }
        std::fs::create_dir_all(config_dir)?;
        std::fs::write(&path, DEFAULT_AUTOMATIONS_TOML)?;
        Ok(())
    }
}

// ─── Cron helper ─────────────────────────────────────────────────────────────

/// Returns true when `schedule` (5-field cron) matches `now` at minute
/// granularity.  Supported patterns per field:
///   `*`     — any value
///   `N`     — exact integer match
///   `*/N`   — every N units (value % N == 0)
pub fn cron_matches(schedule: &str, now: &chrono::DateTime<Local>) -> bool {
    let fields: Vec<&str> = schedule.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }

    let values = [
        now.minute(),
        now.hour(),
        now.day(),
        now.month(),
        now.weekday().num_days_from_sunday(), // 0 = Sunday
    ];

    for (field, &value) in fields.iter().zip(values.iter()) {
        if !field_matches(field, value) {
            return false;
        }
    }
    true
}

fn field_matches(field: &str, value: u32) -> bool {
    if field == "*" {
        return true;
    }
    if let Some(step) = field.strip_prefix("*/") {
        if let Ok(n) = step.parse::<u32>() {
            return n > 0 && value % n == 0;
        }
        return false;
    }
    if let Ok(n) = field.parse::<u32>() {
        return n == value;
    }
    false
}

// ─── Scheduler ───────────────────────────────────────────────────────────────

/// Background task: wakes at the top of every minute, evaluates all
/// automations, and pushes triggered notifications into `notify_queue`.
///
/// Each item in the queue is `(phone_number, message_body)`.
pub async fn run_scheduler(
    agent: Arc<Mutex<Agent>>,
    notify_queue: Arc<Mutex<VecDeque<(String, String)>>>,
    whatsapp_cfg: WhatsAppConfig,
    config_dir: PathBuf,
) {
    // Keep track of the last minute we processed to avoid double-firing.
    let mut last_minute: Option<(u32, u32)> = None; // (hour, minute)

    loop {
        // Sleep until the next top-of-minute boundary (± a few ms)
        let now = Local::now();
        let secs_remaining = 60 - now.second();
        tokio::time::sleep(tokio::time::Duration::from_secs(secs_remaining as u64)).await;

        let now = Local::now();
        let this_minute = (now.hour(), now.minute());

        // Guard against double-fire if the sleep wakes early
        if last_minute == Some(this_minute) {
            continue;
        }
        last_minute = Some(this_minute);

        // Reload config each minute so changes take effect without restart
        let cfg = AutomationsConfig::load(&config_dir);

        for auto in &cfg.automations {
            if !auto.enabled {
                continue;
            }
            if !cron_matches(&auto.schedule, &now) {
                continue;
            }

            eprintln!("[scheduler] Triggering automation: {}", auto.name);

            // Run through the AI agent
            let reply = {
                let mut locked = agent.lock().await;
                locked.run_automation(&auto.action).await
            };

            let message = match reply {
                Ok(text) => format!(
                    "🤖 Blunux AI Agent — {}\n\n{}",
                    auto.name, text
                ),
                Err(e) => format!(
                    "🤖 Blunux AI Agent — {}\n\n⚠️ 자동화 오류: {e}",
                    auto.name
                ),
            };

            // Push to all allowed WhatsApp numbers
            if auto.notify == "whatsapp" && !whatsapp_cfg.allowed_numbers.is_empty() {
                let mut queue = notify_queue.lock().await;
                for phone in &whatsapp_cfg.allowed_numbers {
                    queue.push_back((phone.clone(), message.clone()));
                }
            }
        }
    }
}

// ─── Default config template ─────────────────────────────────────────────────

const DEFAULT_AUTOMATIONS_TOML: &str = r#"# ~/.config/blunux-ai/automations.toml
# 자동화 에이전트 설정 / Automation agent configuration
#
# schedule 형식 (5-field cron):
#   분(0-59) 시(0-23) 일(1-31) 월(1-12) 요일(0-6, 0=일요일)
#   *  = 모두 일치 / any value
#   */N = N마다 / every N units
#
# Examples:
#   "0 9 * * *"   → 매일 오전 9시   / every day at 09:00
#   "0 */6 * * *" → 6시간마다       / every 6 hours
#   "0 0 * * *"   → 매일 자정       / every day at midnight

[[automation]]
name = "시스템 헬스체크"
schedule = "0 9 * * *"
action = "시스템 전체 상태를 확인하고 CPU, RAM, 디스크, 업타임, 보류 중인 업데이트 수를 요약해줘. 문제가 있으면 명확히 알려줘."
notify = "whatsapp"
enabled = true

[[automation]]
name = "보안 업데이트 확인"
schedule = "0 */6 * * *"
action = "보안 업데이트가 있는지 확인해줘. 있으면 패키지 목록과 함께 알려줘. 없으면 '보안 업데이트 없음'이라고 짧게 답해줘."
notify = "whatsapp"
auto_apply = false
enabled = true

[[automation]]
name = "디스크 공간 경고"
schedule = "0 0 * * *"
action = "디스크 사용률을 확인해서 80% 이상인 파티션이 있으면 경고해줘. 모두 안전하면 '디스크 공간 정상'이라고 짧게 답해줘."
notify = "whatsapp"
enabled = true
"#;

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn dt(hour: u32, minute: u32, day: u32) -> chrono::DateTime<Local> {
        // 2026-02-{day} HH:MM:00 local
        Local
            .with_ymd_and_hms(2026, 2, day, hour, minute, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn test_cron_daily_9am() {
        let schedule = "0 9 * * *";
        assert!(cron_matches(schedule, &dt(9, 0, 21)));
        assert!(!cron_matches(schedule, &dt(9, 1, 21)));
        assert!(!cron_matches(schedule, &dt(10, 0, 21)));
    }

    #[test]
    fn test_cron_every_6h() {
        let schedule = "0 */6 * * *";
        assert!(cron_matches(schedule, &dt(0, 0, 21)));
        assert!(cron_matches(schedule, &dt(6, 0, 21)));
        assert!(cron_matches(schedule, &dt(12, 0, 21)));
        assert!(cron_matches(schedule, &dt(18, 0, 21)));
        assert!(!cron_matches(schedule, &dt(3, 0, 21)));
        assert!(!cron_matches(schedule, &dt(6, 1, 21)));
    }

    #[test]
    fn test_cron_midnight() {
        let schedule = "0 0 * * *";
        assert!(cron_matches(schedule, &dt(0, 0, 21)));
        assert!(!cron_matches(schedule, &dt(0, 1, 21)));
        assert!(!cron_matches(schedule, &dt(1, 0, 21)));
    }

    #[test]
    fn test_cron_star_matches_all() {
        assert!(cron_matches("* * * * *", &dt(14, 37, 15)));
    }

    #[test]
    fn test_cron_invalid_field_count() {
        assert!(!cron_matches("0 9 * *", &dt(9, 0, 1))); // only 4 fields
    }

    #[test]
    fn test_automations_load_defaults_on_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = AutomationsConfig::load(tmp.path());
        assert!(cfg.automations.is_empty());
    }

    #[test]
    fn test_automations_write_and_load_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        AutomationsConfig::write_defaults(tmp.path()).unwrap();
        let cfg = AutomationsConfig::load(tmp.path());
        assert_eq!(cfg.automations.len(), 3);
        assert_eq!(cfg.automations[0].name, "시스템 헬스체크");
        assert_eq!(cfg.automations[1].name, "보안 업데이트 확인");
        assert_eq!(cfg.automations[2].name, "디스크 공간 경고");
    }

    #[test]
    fn test_automation_enabled_defaults_true() {
        let tmp = tempfile::tempdir().unwrap();
        AutomationsConfig::write_defaults(tmp.path()).unwrap();
        let cfg = AutomationsConfig::load(tmp.path());
        assert!(cfg.automations.iter().all(|a| a.enabled));
    }
}
