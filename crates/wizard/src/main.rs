mod hwdetect;

use anyhow::Result;
use blunux_config::BlunuxConfig;
use std::path::Path;
use std::process::Command;

const CONFIG_PATH: &str = "/usr/share/blunux/config.toml";

fn main() -> Result<()> {
    println!("╔══════════════════════════════════════╗");
    println!("║     blunux2 Setup Wizard v2.0        ║");
    println!("║     Rust + C  (no Python)            ║");
    println!("╚══════════════════════════════════════╝");
    println!();

    // 1. Hardware detection
    step_hardware_detect();

    // 2. Load config.toml
    let config = step_load_config(CONFIG_PATH)?;

    // 3. Apply live session settings
    step_apply_locale(&config);
    step_apply_keyboard(&config);

    // 4. Launch desktop session
    step_launch_desktop();

    Ok(())
}

fn step_hardware_detect() {
    println!("── Hardware Detection ──");

    let gpu = hwdetect::detect_gpu();
    println!("  GPU: {}", gpu.name());

    let drivers = hwdetect::gpu_driver_packages(gpu);
    println!("  Auto-selected drivers: {}", drivers.join(", "));

    let audio = hwdetect::detect_audio();
    println!("  Audio: {}", audio.name());

    let uefi = hwdetect::is_uefi();
    println!("  Boot mode: {}", if uefi { "UEFI" } else { "BIOS" });

    let ram = hwdetect::total_ram_mb();
    println!("  RAM: {} MB", ram);
}

fn step_load_config(path: &str) -> Result<BlunuxConfig> {
    println!("\n── Loading Configuration ──");

    let config = BlunuxConfig::load(Path::new(path))
        .map_err(|e| anyhow::anyhow!("Failed to load {}: {}", path, e))?;

    println!("  Language: {:?}", config.locale.language);
    println!("  Timezone: {}", config.locale.timezone);
    println!("  Hostname: {}", config.install.hostname);
    println!("  Username: {}", config.install.username);
    println!("  Bootloader: {}", config.install.bootloader);
    println!("  Swap: {}", config.disk.swap);
    println!("  Kernel: {}", config.kernel.kernel_type);

    Ok(config)
}

fn step_apply_locale(config: &BlunuxConfig) {
    println!("\n── Applying Locale ──");

    let lang = config
        .locale
        .language
        .first()
        .map(|s| s.as_str())
        .unwrap_or("en_US");
    let tz = &config.locale.timezone;

    // Apply locale to live session (best-effort, ignore errors)
    let _ = Command::new("localectl")
        .args(["set-locale", &format!("LANG={}.UTF-8", lang)])
        .status();

    let _ = Command::new("timedatectl")
        .args(["set-timezone", tz])
        .status();

    println!("  Applied: LANG={}.UTF-8, TZ={}", lang, tz);
}

fn step_apply_keyboard(config: &BlunuxConfig) {
    println!("\n── Applying Keyboard Layout ──");

    if let Some(layout) = config.locale.keyboard.first() {
        let _ = Command::new("localectl")
            .args(["set-x11-keymap", layout])
            .status();
        println!("  Applied: {}", layout);
    } else {
        println!("  No keyboard layout specified, using default");
    }
}

fn step_launch_desktop() {
    println!("\n── Launching Desktop ──");
    println!("  Starting Plasma Wayland session...");

    // Replace this process with the desktop session.
    // exec replaces the current process — this binary exits, Plasma takes over.
    let err = exec_replace("startplasma-wayland");

    // If exec returns, Wayland failed — try X11
    eprintln!("  Wayland failed ({}), falling back to X11...", err);
    let err = exec_replace("startplasma-x11");

    eprintln!("  X11 also failed ({}). No desktop session available.", err);
    std::process::exit(1);
}

/// Replace the current process with the given command (unix exec).
fn exec_replace(cmd: &str) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    // This only returns if exec fails
    Command::new(cmd).exec()
}
