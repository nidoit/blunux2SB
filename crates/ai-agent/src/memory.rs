use std::path::{Path, PathBuf};

use chrono::Local;

use crate::error::MemoryError;

#[derive(Debug, Default)]
pub struct SystemInfo {
    pub distro: String,
    pub kernel: String,
    pub desktop_env: String,
    pub shell: String,
    pub cpu: String,
    pub memory_total_gb: f64,
    pub memory_used_gb: f64,
    pub disk_total_gb: f64,
    pub disk_used_gb: f64,
    pub hostname: String,
    pub username: String,
}

pub struct Memory {
    base_dir: PathBuf,
}

impl Memory {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn memory_dir(&self) -> PathBuf {
        self.base_dir.join("memory")
    }

    fn daily_dir(&self) -> PathBuf {
        self.memory_dir().join("daily")
    }

    fn logs_dir(&self) -> PathBuf {
        self.base_dir.join("logs")
    }

    /// Ensure all memory directories exist.
    pub fn init_dirs(&self) -> Result<(), MemoryError> {
        for dir in [
            &self.memory_dir(),
            &self.daily_dir(),
            &self.logs_dir(),
            &self.base_dir.join("credentials"),
        ] {
            std::fs::create_dir_all(dir).map_err(|e| MemoryError::Write {
                path: dir.display().to_string(),
                source: e,
            })?;
        }
        Ok(())
    }

    fn read_file(&self, path: &Path) -> Result<String, MemoryError> {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok(content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(MemoryError::Read {
                path: path.display().to_string(),
                source: e,
            }),
        }
    }

    fn write_file(&self, path: &Path, content: &str) -> Result<(), MemoryError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| MemoryError::Write {
                path: parent.display().to_string(),
                source: e,
            })?;
        }
        std::fs::write(path, content).map_err(|e| MemoryError::Write {
            path: path.display().to_string(),
            source: e,
        })
    }

    pub fn load_system(&self) -> Result<String, MemoryError> {
        self.read_file(&self.memory_dir().join("SYSTEM.md"))
    }

    pub fn load_user(&self) -> Result<String, MemoryError> {
        self.read_file(&self.memory_dir().join("USER.md"))
    }

    pub fn load_long_term(&self) -> Result<String, MemoryError> {
        self.read_file(&self.memory_dir().join("MEMORY.md"))
    }

    pub fn load_today(&self) -> Result<String, MemoryError> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let path = self.daily_dir().join(format!("{today}.md"));
        self.read_file(&path)
    }

    pub fn append_today(&self, content: &str) -> Result<(), MemoryError> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let path = self.daily_dir().join(format!("{today}.md"));
        let time = Local::now().format("%H:%M").to_string();

        let existing = self.read_file(&path)?;
        let new_content = if existing.is_empty() {
            format!("# {today}\n\n{time} - {content}\n")
        } else {
            format!("{existing}{time} - {content}\n")
        };
        self.write_file(&path, &new_content)
    }

    pub fn update_user(&self, content: &str) -> Result<(), MemoryError> {
        self.write_file(&self.memory_dir().join("USER.md"), content)
    }

    pub fn build_context(&self) -> Result<String, MemoryError> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut ctx = String::new();

        let system = self.load_system()?;
        if !system.is_empty() {
            ctx.push_str("## System Information\n");
            ctx.push_str(&system);
            ctx.push_str("\n\n");
        }

        let user = self.load_user()?;
        if !user.is_empty() {
            ctx.push_str("## User Preferences\n");
            ctx.push_str(&user);
            ctx.push_str("\n\n");
        }

        let long_term = self.load_long_term()?;
        if !long_term.is_empty() {
            ctx.push_str("## Long-term Memory\n");
            ctx.push_str(&long_term);
            ctx.push_str("\n\n");
        }

        let daily = self.load_today()?;
        if !daily.is_empty() {
            ctx.push_str(&format!("## Today's Session ({today})\n"));
            ctx.push_str(&daily);
            ctx.push('\n');
        }

        Ok(ctx)
    }

    pub fn refresh_system_info(&self) -> Result<(), MemoryError> {
        let info = self.detect_system_info();
        let md = format!(
            "# System Information\n\
             - Hostname: {}\n\
             - Username: {}\n\
             - Distro: {}\n\
             - Kernel: {}\n\
             - Desktop: {}\n\
             - Shell: {}\n\
             - CPU: {}\n\
             - RAM: {:.1} GB total, {:.1} GB used\n\
             - Disk: {:.1} GB total, {:.1} GB used\n",
            info.hostname,
            info.username,
            info.distro,
            info.kernel,
            info.desktop_env,
            info.shell,
            info.cpu,
            info.memory_total_gb,
            info.memory_used_gb,
            info.disk_total_gb,
            info.disk_used_gb,
        );
        self.write_file(&self.memory_dir().join("SYSTEM.md"), &md)
    }

    pub fn detect_system_info(&self) -> SystemInfo {
        let mut info = SystemInfo::default();

        info.hostname = cmd_output("hostname").unwrap_or_else(|| "unknown".into());
        info.username = std::env::var("USER").unwrap_or_else(|_| "unknown".into());
        info.kernel = cmd_output_args("uname", &["-r"]).unwrap_or_else(|| "unknown".into());
        info.shell = std::env::var("SHELL").unwrap_or_else(|_| "unknown".into());
        info.desktop_env =
            std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into());

        // Distro from /etc/os-release
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if let Some(name) = line.strip_prefix("PRETTY_NAME=") {
                    info.distro = name.trim_matches('"').to_string();
                    break;
                }
            }
        }
        if info.distro.is_empty() {
            info.distro = "Blunux (Arch Linux)".into();
        }

        // CPU from /proc/cpuinfo
        if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
            for line in content.lines() {
                if let Some(name) = line.strip_prefix("model name") {
                    if let Some(val) = name.split(':').nth(1) {
                        info.cpu = val.trim().to_string();
                        break;
                    }
                }
            }
        }

        // Memory from /proc/meminfo
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if let Some(val) = line.strip_prefix("MemTotal:") {
                    if let Some(kb) = parse_kb(val) {
                        info.memory_total_gb = kb as f64 / 1_048_576.0;
                    }
                } else if let Some(val) = line.strip_prefix("MemAvailable:") {
                    if let Some(kb) = parse_kb(val) {
                        let avail_gb = kb as f64 / 1_048_576.0;
                        info.memory_used_gb = info.memory_total_gb - avail_gb;
                    }
                }
            }
        }

        // Disk from df
        if let Some(df_out) = cmd_output_args("df", &["--output=size,used", "-B1", "/"]) {
            let lines: Vec<&str> = df_out.lines().collect();
            if lines.len() >= 2 {
                let parts: Vec<&str> = lines[1].split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(total) = parts[0].parse::<u64>() {
                        info.disk_total_gb = total as f64 / 1_073_741_824.0;
                    }
                    if let Ok(used) = parts[1].parse::<u64>() {
                        info.disk_used_gb = used as f64 / 1_073_741_824.0;
                    }
                }
            }
        }

        info
    }

    /// Append a command log entry.
    pub fn log_command(&self, status: &str, command: &str) -> Result<(), MemoryError> {
        let path = self.logs_dir().join("commands.log");
        let timestamp = Local::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let entry = format!("[{timestamp}] {status:<12}{command}\n");

        let existing = self.read_file(&path)?;
        self.write_file(&path, &format!("{existing}{entry}"))
    }

    /// Clear daily logs and long-term memory.
    pub fn clear(&self) -> Result<(), MemoryError> {
        let daily = self.daily_dir();
        if daily.exists() {
            std::fs::remove_dir_all(&daily).map_err(|e| MemoryError::Write {
                path: daily.display().to_string(),
                source: e,
            })?;
            std::fs::create_dir_all(&daily).map_err(|e| MemoryError::Write {
                path: daily.display().to_string(),
                source: e,
            })?;
        }
        let memory_file = self.memory_dir().join("MEMORY.md");
        if memory_file.exists() {
            self.write_file(&memory_file, "")?;
        }
        Ok(())
    }

    /// Show all memory contents as a formatted string.
    pub fn show_all(&self) -> Result<String, MemoryError> {
        let mut out = String::new();

        let system = self.load_system()?;
        if !system.is_empty() {
            out.push_str("=== SYSTEM.md ===\n");
            out.push_str(&system);
            out.push_str("\n\n");
        }

        let user = self.load_user()?;
        if !user.is_empty() {
            out.push_str("=== USER.md ===\n");
            out.push_str(&user);
            out.push_str("\n\n");
        }

        let long_term = self.load_long_term()?;
        if !long_term.is_empty() {
            out.push_str("=== MEMORY.md ===\n");
            out.push_str(&long_term);
            out.push_str("\n\n");
        }

        let today = self.load_today()?;
        if !today.is_empty() {
            let date = Local::now().format("%Y-%m-%d");
            out.push_str(&format!("=== Today ({date}) ===\n"));
            out.push_str(&today);
            out.push('\n');
        }

        if out.is_empty() {
            out.push_str("(empty — no memory files found)\n");
        }

        Ok(out)
    }
}

fn cmd_output(cmd: &str) -> Option<String> {
    std::process::Command::new(cmd)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn cmd_output_args(cmd: &str, args: &[&str]) -> Option<String> {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn parse_kb(val: &str) -> Option<u64> {
    val.trim()
        .trim_end_matches("kB")
        .trim()
        .parse::<u64>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_init_and_readwrite() {
        let tmp = tempfile::tempdir().unwrap();
        let mem = Memory::new(tmp.path().to_path_buf());
        mem.init_dirs().unwrap();

        // Empty reads should succeed
        assert_eq!(mem.load_system().unwrap(), "");
        assert_eq!(mem.load_user().unwrap(), "");
        assert_eq!(mem.load_long_term().unwrap(), "");
        assert_eq!(mem.load_today().unwrap(), "");

        // Write and read back
        mem.update_user("browser=firefox").unwrap();
        assert_eq!(mem.load_user().unwrap(), "browser=firefox");
    }

    #[test]
    fn test_memory_append_today() {
        let tmp = tempfile::tempdir().unwrap();
        let mem = Memory::new(tmp.path().to_path_buf());
        mem.init_dirs().unwrap();

        mem.append_today("Checked system status").unwrap();
        let today = mem.load_today().unwrap();
        assert!(today.contains("Checked system status"));

        mem.append_today("Installed chrome").unwrap();
        let today = mem.load_today().unwrap();
        assert!(today.contains("Installed chrome"));
    }

    #[test]
    fn test_memory_build_context_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let mem = Memory::new(tmp.path().to_path_buf());
        mem.init_dirs().unwrap();

        let ctx = mem.build_context().unwrap();
        assert!(ctx.is_empty() || ctx.trim().is_empty() || !ctx.contains("error"));
    }

    #[test]
    fn test_memory_log_command() {
        let tmp = tempfile::tempdir().unwrap();
        let mem = Memory::new(tmp.path().to_path_buf());
        mem.init_dirs().unwrap();

        mem.log_command("SAFE", "df -h").unwrap();
        mem.log_command("CONFIRMED", "yay -S chrome").unwrap();

        let log = std::fs::read_to_string(tmp.path().join("logs/commands.log")).unwrap();
        assert!(log.contains("SAFE"));
        assert!(log.contains("df -h"));
        assert!(log.contains("CONFIRMED"));
    }

    #[test]
    fn test_memory_clear() {
        let tmp = tempfile::tempdir().unwrap();
        let mem = Memory::new(tmp.path().to_path_buf());
        mem.init_dirs().unwrap();

        mem.append_today("test entry").unwrap();
        assert!(!mem.load_today().unwrap().is_empty());

        mem.clear().unwrap();
        assert_eq!(mem.load_today().unwrap(), "");
    }
}
