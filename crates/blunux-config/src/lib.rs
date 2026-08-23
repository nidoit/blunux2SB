use serde::{Deserialize, Serialize};
use std::path::Path;

/// Root configuration — mirrors config.toml structure exactly.
#[derive(Debug, Deserialize, Serialize)]
pub struct BlunuxConfig {
    pub blunux: BlunuxMeta,
    pub locale: Locale,
    pub input_method: InputMethod,
    pub kernel: Kernel,
    pub install: Install,
    pub disk: Disk,
    pub packages: Packages,
    #[serde(default)]
    pub ai_agent: Option<AiAgent>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AiAgent {
    pub enabled: bool,
    pub provider: String,
    pub claude_mode: String,
    pub whatsapp_enabled: bool,
    pub language: String,
    pub safe_mode: bool,
}

impl Default for AiAgent {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "claude".into(),
            claude_mode: "oauth".into(),
            whatsapp_enabled: false,
            language: "auto".into(),
            safe_mode: true,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlunuxMeta {
    pub version: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Locale {
    pub language: Vec<String>,
    pub timezone: String,
    pub keyboard: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InputMethod {
    pub enabled: bool,
    pub engine: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Kernel {
    #[serde(rename = "type")]
    pub kernel_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Install {
    pub bootloader: String,
    pub hostname: String,
    pub username: String,
    pub root_password: String,
    pub user_password: String,
    pub encryption: bool,
    pub autologin: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Disk {
    pub swap: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Packages {
    pub desktop: DesktopPkgs,
    pub browser: BrowserPkgs,
    pub office: OfficePkgs,
    pub development: DevelopmentPkgs,
    pub multimedia: MultimediaPkgs,
    pub gaming: GamingPkgs,
    pub virtualization: VirtualizationPkgs,
    pub communication: CommunicationPkgs,
    pub utility: UtilityPkgs,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DesktopPkgs {
    pub kde: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BrowserPkgs {
    pub firefox: bool,
    pub whale: bool,
    pub chrome: bool,
    pub mullvad: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OfficePkgs {
    pub libreoffice: bool,
    pub hoffice: bool,
    pub texlive: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DevelopmentPkgs {
    pub vscode: bool,
    pub sublime: bool,
    pub rust: bool,
    pub julia: bool,
    pub nodejs: bool,
    pub github_cli: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MultimediaPkgs {
    pub obs: bool,
    pub vlc: bool,
    pub freetv: bool,
    pub ytdlp: bool,
    pub freetube: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GamingPkgs {
    pub steam: bool,
    pub unciv: bool,
    pub snes9x: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VirtualizationPkgs {
    pub virtualbox: bool,
    pub docker: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommunicationPkgs {
    pub teams: bool,
    pub whatsapp: bool,
    pub onenote: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UtilityPkgs {
    pub conky: bool,
    pub vnc: bool,
    pub samba: bool,
    pub bluetooth: bool,
}

impl BlunuxConfig {
    /// Load config from a TOML file path.
    ///
    /// Validation runs here so every consumer (build, wizard, setup, toml2cal)
    /// is protected: several of these values end up inside root shell commands
    /// and Calamares YAML during real disk installation.
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let contents = std::fs::read_to_string(path)?;
        let config: BlunuxConfig = toml::from_str(&contents)?;
        config.validate().map_err(|errors| {
            format!(
                "invalid config.toml ({}):\n  - {}",
                path.display(),
                errors.join("\n  - ")
            )
        })?;
        Ok(config)
    }

    /// Check every field that flows into shell commands or generated YAML.
    /// Returns all problems at once so the user can fix them in one pass.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        const KERNELS: &[&str] = &["linux", "linux-lts", "linux-zen", "linux-hardened"];
        const BOOTLOADERS: &[&str] = &["grub", "systemd-boot", "nmbl"];
        const SWAPS: &[&str] = &["none", "small", "suspend", "file"];
        const ENGINES: &[&str] = &["kime", "fcitx5", "ibus"];

        let mut errors = Vec::new();

        if !KERNELS.contains(&self.kernel.kernel_type.as_str()) {
            errors.push(format!(
                "kernel.type = {:?} — must be one of: {}",
                self.kernel.kernel_type,
                KERNELS.join(", ")
            ));
        }
        if !BOOTLOADERS.contains(&self.install.bootloader.as_str()) {
            errors.push(format!(
                "install.bootloader = {:?} — must be one of: {}",
                self.install.bootloader,
                BOOTLOADERS.join(", ")
            ));
        }
        if !SWAPS.contains(&self.disk.swap.as_str()) {
            errors.push(format!(
                "disk.swap = {:?} — must be one of: {}",
                self.disk.swap,
                SWAPS.join(", ")
            ));
        }
        if self.input_method.enabled && !ENGINES.contains(&self.input_method.engine.as_str()) {
            errors.push(format!(
                "input_method.engine = {:?} — must be one of: {}",
                self.input_method.engine,
                ENGINES.join(", ")
            ));
        }
        if !valid_username(&self.install.username) {
            errors.push(format!(
                "install.username = {:?} — must match a POSIX username: \
                 lowercase letter or '_' first, then lowercase/digits/'_'/'-', max 32 chars",
                self.install.username
            ));
        }
        if !valid_hostname(&self.install.hostname) {
            errors.push(format!(
                "install.hostname = {:?} — letters, digits and '-' only, \
                 no leading/trailing '-', max 63 chars",
                self.install.hostname
            ));
        }
        for lang in &self.locale.language {
            if !valid_locale(lang) {
                errors.push(format!(
                    "locale.language entry {lang:?} — expected a locale code like \"ko_KR\""
                ));
            }
        }
        if !valid_timezone(&self.locale.timezone) {
            errors.push(format!(
                "locale.timezone = {:?} — expected an IANA zone like \"Europe/Stockholm\"",
                self.locale.timezone
            ));
        }
        for kb in &self.locale.keyboard {
            if !(1..=8).contains(&kb.len()) || !kb.chars().all(|c| c.is_ascii_alphanumeric()) {
                errors.push(format!(
                    "locale.keyboard entry {kb:?} — expected a short layout code like \"kr\""
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Save config back to a TOML file path.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}

fn valid_username(s: &str) -> bool {
    let mut chars = s.chars();
    let first_ok = matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_');
    first_ok
        && s.len() <= 32
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn valid_hostname(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 63
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !s.starts_with('-')
        && !s.ends_with('-')
}

fn valid_locale(s: &str) -> bool {
    !s.is_empty() && s.len() <= 16 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn valid_timezone(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !s.starts_with('/')
        && !s.ends_with('/')
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '+' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_config() {
        let toml_str = r#"
[blunux]
version = "2.0"
name = "test-build"

[locale]
language = ["ko_KR"]
timezone = "Europe/Stockholm"
keyboard = ["kr", "us"]

[input_method]
enabled = true
engine = "kime"

[kernel]
type = "linux"

[install]
bootloader = "systemd-boot"
hostname = "nux"
username = "blu"
root_password = "1234"
user_password = "1234"
encryption = false
autologin = true

[disk]
swap = "suspend"

[packages.desktop]
kde = true

[packages.browser]
firefox = true
whale = false
chrome = false
mullvad = false

[packages.office]
libreoffice = true
hoffice = false
texlive = false

[packages.development]
vscode = true
sublime = false
rust = true
julia = true
nodejs = true
github_cli = false

[packages.multimedia]
obs = false
vlc = false
freetv = false
ytdlp = false
freetube = false

[packages.gaming]
steam = false
unciv = false
snes9x = false

[packages.virtualization]
virtualbox = false
docker = false

[packages.communication]
teams = false
whatsapp = false
onenote = false

[packages.utility]
conky = false
vnc = false
samba = false
bluetooth = true
"#;
        let config: BlunuxConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.blunux.version, "2.0");
        assert_eq!(config.install.bootloader, "systemd-boot");
        assert_eq!(config.disk.swap, "suspend");
        assert!(config.packages.desktop.kde);
        assert!(config.packages.browser.firefox);
        assert!(!config.packages.gaming.steam);
        config.validate().unwrap();
    }

    fn valid_config() -> BlunuxConfig {
        toml::from_str(
            r#"
[blunux]
version = "2.0"
name = "test-build"
[locale]
language = ["ko_KR"]
timezone = "Europe/Stockholm"
keyboard = ["kr", "us"]
[input_method]
enabled = true
engine = "kime"
[kernel]
type = "linux"
[install]
bootloader = "systemd-boot"
hostname = "nux"
username = "blu"
root_password = "1234"
user_password = "1234"
encryption = false
autologin = true
[disk]
swap = "suspend"
[packages.desktop]
kde = true
[packages.browser]
firefox = true
whale = false
chrome = false
mullvad = false
[packages.office]
libreoffice = false
hoffice = false
texlive = false
[packages.development]
vscode = false
sublime = false
rust = false
julia = false
nodejs = false
github_cli = false
[packages.multimedia]
obs = false
vlc = false
freetv = false
ytdlp = false
freetube = false
[packages.gaming]
steam = false
unciv = false
snes9x = false
[packages.virtualization]
virtualbox = false
docker = false
[packages.communication]
teams = false
whatsapp = false
onenote = false
[packages.utility]
conky = false
vnc = false
samba = false
bluetooth = false
"#,
        )
        .unwrap()
    }

    // Injection regression tests: these values reach root shell commands
    // (toml2cal shellprocess) and Calamares YAML — they must never validate.

    #[test]
    fn test_validate_rejects_kernel_injection() {
        let mut cfg = valid_config();
        cfg.kernel.kernel_type = "linux; rm -rf /".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_kernel_unknown() {
        let mut cfg = valid_config();
        cfg.kernel.kernel_type = "linux-rt".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_username_injection() {
        for bad in [
            "blu\"; rm -rf /; \"",
            "blu name",
            "blu$(reboot)",
            "BLU",
            "-blu",
            "blu\nroot",
            "",
        ] {
            let mut cfg = valid_config();
            cfg.install.username = bad.into();
            assert!(cfg.validate().is_err(), "username {bad:?} must be rejected");
        }
    }

    #[test]
    fn test_validate_rejects_hostname_injection() {
        for bad in ["host\"name", "host name", "host;reboot", "-host", ""] {
            let mut cfg = valid_config();
            cfg.install.hostname = bad.into();
            assert!(cfg.validate().is_err(), "hostname {bad:?} must be rejected");
        }
    }

    #[test]
    fn test_validate_rejects_bad_bootloader_swap_engine() {
        let mut cfg = valid_config();
        cfg.install.bootloader = "grub2; id".into();
        assert!(cfg.validate().is_err());

        let mut cfg = valid_config();
        cfg.disk.swap = "small'".into();
        assert!(cfg.validate().is_err());

        let mut cfg = valid_config();
        cfg.input_method.engine = "kime && reboot".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_engine_ignored_when_disabled() {
        let mut cfg = valid_config();
        cfg.input_method.enabled = false;
        cfg.input_method.engine = "whatever".into();
        cfg.validate().unwrap();
    }

    #[test]
    fn test_validate_rejects_locale_injection() {
        let mut cfg = valid_config();
        cfg.locale.language = vec!["ko_KR\"'".into()];
        assert!(cfg.validate().is_err());

        let mut cfg = valid_config();
        cfg.locale.timezone = "Europe/../../etc".into();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_repo_config_toml_passes_validation() {
        // The shipped default config must always load (validation included).
        let path = std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config.toml"
        ));
        BlunuxConfig::load(path).expect("repo config.toml must be valid");
    }
}
