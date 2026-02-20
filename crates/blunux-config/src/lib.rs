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
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let contents = std::fs::read_to_string(path)?;
        let config: BlunuxConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save config back to a TOML file path.
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
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
    }
}
