mod generate;
mod packages;

use anyhow::{Context, Result};
use blunux_config::BlunuxConfig;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "blunux-toml2cal")]
#[command(about = "Translate config.toml into Calamares YAML configuration files")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate all Calamares YAML configs from config.toml
    Generate {
        /// Path to config.toml
        #[arg(short, long)]
        input: PathBuf,

        /// Directory to write Calamares module configs into
        #[arg(short, long, default_value = "/etc/calamares/modules")]
        output_dir: PathBuf,

        /// Path to write settings.conf
        #[arg(short, long, default_value = "/etc/calamares/settings.conf")]
        settings: PathBuf,
    },

    /// Install packages listed in config.toml [packages.*] sections via pacman
    ApplyPackages {
        /// Path to config.toml
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Configure input method from config.toml [input_method]
    ApplyInputMethod {
        /// Path to config.toml
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            input,
            output_dir,
            settings,
        } => cmd_generate(&input, &output_dir, &settings),
        Commands::ApplyPackages { input } => cmd_apply_packages(&input),
        Commands::ApplyInputMethod { input } => cmd_apply_input_method(&input),
    }
}

fn load_config(input: &Path) -> Result<BlunuxConfig> {
    BlunuxConfig::load(input)
        .map_err(|e| anyhow::anyhow!("Failed to load config from {}: {}", input.display(), e))
}

fn cmd_generate(input: &Path, output_dir: &Path, settings_path: &Path) -> Result<()> {
    let config = load_config(input)?;

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output dir {}", output_dir.display()))?;

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Generate settings.conf (module pipeline)
    let settings_yaml = generate::settings_conf(&config);
    std::fs::write(settings_path, &settings_yaml)
        .with_context(|| format!("Failed to write {}", settings_path.display()))?;
    eprintln!("Wrote {}", settings_path.display());

    // Generate per-module configs
    let modules: Vec<(&str, String)> = vec![
        ("locale.conf", generate::locale_conf(&config)),
        ("keyboard.conf", generate::keyboard_conf(&config)),
        ("partition.conf", generate::partition_conf(&config)),
        ("users.conf", generate::users_conf(&config)),
        ("bootloader.conf", generate::bootloader_conf(&config)),
        ("unpackfs.conf", generate::unpackfs_conf()),
        ("shellprocess.conf", generate::shellprocess_conf(&config)),
        (
            "services-systemd.conf",
            generate::services_systemd_conf(&config),
        ),
        (
            "displaymanager.conf",
            generate::displaymanager_conf(&config),
        ),
    ];

    for (filename, content) in &modules {
        let path = output_dir.join(filename);
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write {}", path.display()))?;
        eprintln!("Wrote {}", path.display());
    }

    eprintln!(
        "Generated {} config files from {}",
        modules.len() + 1,
        input.display()
    );
    Ok(())
}

fn cmd_apply_packages(input: &Path) -> Result<()> {
    let config = load_config(input)?;

    let pkgs = packages::resolve(&config);
    if pkgs.is_empty() {
        eprintln!("No additional packages to install.");
        return Ok(());
    }

    eprintln!("Installing {} packages: {}", pkgs.len(), pkgs.join(" "));

    // Use yay if available (handles AUR), fall back to pacman
    let pkg_mgr = if has_cmd("yay") { "yay" } else { "pacman" };
    let status = std::process::Command::new(pkg_mgr)
        .args(["-S", "--noconfirm", "--needed"])
        .args(&pkgs)
        .status()
        .with_context(|| format!("Failed to run {}", pkg_mgr))?;

    if !status.success() {
        anyhow::bail!("{} exited with status {}", pkg_mgr, status);
    }
    Ok(())
}

fn cmd_apply_input_method(input: &Path) -> Result<()> {
    let config = load_config(input)?;

    if !config.input_method.enabled {
        eprintln!("Input method disabled in config, skipping.");
        return Ok(());
    }

    let im_pkgs = match config.input_method.engine.as_str() {
        "kime" => vec!["kime"],
        "fcitx5" => vec!["fcitx5", "fcitx5-hangul", "fcitx5-gtk", "fcitx5-qt", "fcitx5-configtool"],
        "ibus" => vec!["ibus", "ibus-hangul"],
        other => {
            anyhow::bail!("Unknown input method engine: {}", other);
        }
    };

    eprintln!("Installing input method ({}): {}", config.input_method.engine, im_pkgs.join(" "));

    let pkg_mgr = if has_cmd("yay") { "yay" } else { "pacman" };
    let status = std::process::Command::new(pkg_mgr)
        .args(["-S", "--noconfirm", "--needed"])
        .args(&im_pkgs)
        .status()
        .with_context(|| format!("Failed to run {}", pkg_mgr))?;

    if !status.success() {
        anyhow::bail!("pacman exited with status {}", status);
    }

    // Write environment variables for the input method
    let env_content = match config.input_method.engine.as_str() {
        "kime" => "GTK_IM_MODULE=kime\nQT_IM_MODULE=kime\nXMODIFIERS=@im=kime\n",
        "fcitx5" => "GTK_IM_MODULE=fcitx\nQT_IM_MODULE=fcitx\nXMODIFIERS=@im=fcitx\n",
        "ibus" => "GTK_IM_MODULE=ibus\nQT_IM_MODULE=ibus\nXMODIFIERS=@im=ibus\n",
        _ => "",
    };

    if !env_content.is_empty() {
        std::fs::write("/etc/environment.d/input-method.conf", env_content)
            .context("Failed to write input method environment config")?;
        eprintln!("Wrote /etc/environment.d/input-method.conf");
    }

    Ok(())
}

fn has_cmd(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
