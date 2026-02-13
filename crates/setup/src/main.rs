mod packages;

use anyhow::{bail, Context, Result};
use blunux_config::BlunuxConfig;
use clap::Parser;
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "blunux-setup")]
#[command(about = "Install AUR packages and configure blunux2 from config.toml")]
struct Cli {
    /// Path to config.toml
    #[arg(short, long, default_value = "/usr/share/blunux/config.toml")]
    config: PathBuf,

    /// Live ISO mode: also install calamares
    #[arg(long)]
    live: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    println!("blunux-setup: loading {}", cli.config.display());
    let config = BlunuxConfig::load(&cli.config)
        .map_err(|e| anyhow::anyhow!("{}: {}", cli.config.display(), e))?;

    // 1. Bootstrap yay (AUR helper)
    ensure_yay()?;

    // 2. Live mode: install calamares from AUR
    if cli.live {
        step_install_calamares()?;
    }

    // 3. Install user-selected packages (official + AUR, all via yay)
    step_install_packages(&config)?;

    // 4. Input method
    if config.input_method.enabled {
        step_setup_input_method(&config)?;
    }

    // 5. Enable services
    step_enable_services(&config)?;

    println!("\nblunux-setup: done");
    Ok(())
}

// ── yay bootstrap ──────────────────────────────────────────────────────────

fn ensure_yay() -> Result<()> {
    if has_cmd("yay") {
        println!("  yay: found");
        return Ok(());
    }

    println!("── Installing yay ──");

    // base-devel + git needed for makepkg
    sudo_pacman(&["base-devel", "git"])?;

    let tmp = "/tmp/blunux-yay-build";
    let _ = std::fs::remove_dir_all(tmp);

    let status = Command::new("git")
        .args(["clone", "https://aur.archlinux.org/yay-bin.git", tmp])
        .status()
        .context("git clone yay-bin")?;
    if !status.success() {
        bail!("Failed to clone yay-bin");
    }

    let status = Command::new("makepkg")
        .args(["-si", "--noconfirm"])
        .current_dir(tmp)
        .status()
        .context("makepkg yay-bin")?;
    if !status.success() {
        bail!("Failed to build yay-bin");
    }

    let _ = std::fs::remove_dir_all(tmp);
    println!("  yay: installed");
    Ok(())
}

// ── Package installation ───────────────────────────────────────────────────

fn step_install_calamares() -> Result<()> {
    println!("\n── Installing Calamares (live ISO) ──");
    yay_install(&["calamares", "calamares-extensions"])
}

fn step_install_packages(config: &BlunuxConfig) -> Result<()> {
    let pkgs = packages::resolve(config);
    if pkgs.is_empty() {
        println!("\n── No additional packages to install ──");
        return Ok(());
    }

    println!("\n── Installing {} packages ──", pkgs.len());
    for pkg in &pkgs {
        eprint!("  {pkg} ");
    }
    eprintln!();

    let refs: Vec<&str> = pkgs.iter().map(|s| s.as_str()).collect();
    yay_install(&refs)
}

// ── Input method ───────────────────────────────────────────────────────────

fn step_setup_input_method(config: &BlunuxConfig) -> Result<()> {
    let engine = &config.input_method.engine;
    println!("\n── Configuring input method: {engine} ──");

    match engine.as_str() {
        "kime" => setup_kime()?,
        "fcitx5" => setup_fcitx5()?,
        "ibus" => setup_ibus()?,
        other => bail!("Unknown input method engine: {other}"),
    }

    Ok(())
}

fn setup_kime() -> Result<()> {
    // Install kime from AUR
    yay_install(&["kime"])?;

    // Write kime config
    let config_dir = dirs_config().join("kime");
    std::fs::create_dir_all(&config_dir)?;

    let kime_yaml = r#"daemon:
  modules:
    - Xim
    - Wayland
    - Indicator

indicator:
  icon_color: Black

engine:
  hangul:
    layout: dubeolsik
    global_hotkeys:
    - keys: [Hangul]
      behavior: ToggleInputMethod
    - keys: [AltR]
      behavior: ToggleInputMethod
    - keys: [Muhenkan]
      behavior: ToggleInputMethod
  mode:
    hanja_keys: [F9, Hangul_Hanja]
"#;

    std::fs::write(config_dir.join("config.yaml"), kime_yaml)
        .context("write kime config.yaml")?;
    println!("  Wrote kime config.yaml");

    // Environment variables
    write_input_env("kime")?;

    // Autostart desktop entry
    let autostart_dir = dirs_config().join("autostart");
    std::fs::create_dir_all(&autostart_dir)?;
    std::fs::write(
        autostart_dir.join("kime.desktop"),
        "[Desktop Entry]\nName=Kime\nExec=kime\nType=Application\nX-GNOME-Autostart-enabled=true\n",
    )
    .context("write kime autostart")?;
    println!("  Created kime autostart entry");

    Ok(())
}

fn setup_fcitx5() -> Result<()> {
    yay_install(&[
        "fcitx5",
        "fcitx5-hangul",
        "fcitx5-gtk",
        "fcitx5-qt",
        "fcitx5-configtool",
    ])?;
    write_input_env("fcitx")?;
    Ok(())
}

fn setup_ibus() -> Result<()> {
    yay_install(&["ibus", "ibus-hangul"])?;
    write_input_env("ibus")?;
    Ok(())
}

fn write_input_env(module: &str) -> Result<()> {
    // /etc/environment.d/ for systemd environments
    let env_dir = PathBuf::from("/etc/environment.d");
    if env_dir.exists() || sudo_mkdir(&env_dir).is_ok() {
        let content = format!(
            "GTK_IM_MODULE={module}\nQT_IM_MODULE={module}\nXMODIFIERS=@im={module}\n"
        );
        // Write via sudo since /etc is root-owned
        let tmp = "/tmp/blunux-im-env";
        std::fs::write(tmp, &content)?;
        let _ = Command::new("sudo")
            .args(["cp", tmp, "/etc/environment.d/input-method.conf"])
            .status();
        let _ = std::fs::remove_file(tmp);
        println!("  Wrote /etc/environment.d/input-method.conf");
    }

    // Also write to user profile for Xorg sessions
    let home = home_dir();
    let profile_content = format!(
        "\n# Input method\nexport GTK_IM_MODULE={module}\nexport QT_IM_MODULE={module}\nexport XMODIFIERS=@im={module}\n"
    );

    for file in [".bash_profile", ".xprofile"] {
        let path = home.join(file);
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existing.contains("GTK_IM_MODULE") {
            let mut full = existing;
            full.push_str(&profile_content);
            std::fs::write(&path, full)
                .with_context(|| format!("write {}", path.display()))?;
            println!("  Updated ~/{file}");
        }
    }

    Ok(())
}

// ── Services ───────────────────────────────────────────────────────────────

fn step_enable_services(config: &BlunuxConfig) -> Result<()> {
    println!("\n── Enabling services ──");

    let mut services = vec!["NetworkManager"];

    if config.packages.desktop.kde {
        services.push("sddm");
    }
    if config.packages.utility.bluetooth {
        services.push("bluetooth");
    }
    if config.packages.virtualization.docker {
        services.push("docker");
    }

    for svc in &services {
        let status = Command::new("sudo")
            .args(["systemctl", "enable", svc])
            .status();
        match status {
            Ok(s) if s.success() => println!("  Enabled {svc}"),
            _ => eprintln!("  Warning: could not enable {svc}"),
        }
    }

    Ok(())
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn has_cmd(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn sudo_pacman(pkgs: &[&str]) -> Result<()> {
    let status = Command::new("sudo")
        .args(["pacman", "-S", "--noconfirm", "--needed"])
        .args(pkgs)
        .status()
        .context("sudo pacman")?;
    if !status.success() {
        bail!("pacman exited {status}");
    }
    Ok(())
}

fn yay_install(pkgs: &[&str]) -> Result<()> {
    if pkgs.is_empty() {
        return Ok(());
    }
    let status = Command::new("yay")
        .args(["-S", "--noconfirm", "--needed"])
        .args(pkgs)
        .status()
        .context("yay")?;
    if !status.success() {
        bail!("yay exited {status}");
    }
    Ok(())
}

fn sudo_mkdir(path: &std::path::Path) -> Result<()> {
    let status = Command::new("sudo")
        .args(["mkdir", "-p"])
        .arg(path)
        .status()?;
    if !status.success() {
        bail!("mkdir {}", path.display());
    }
    Ok(())
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/root"))
}

fn dirs_config() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home_dir().join(".config"))
}
