use blunux_config::BlunuxConfig;

/// Resolve config.toml booleans into package names.
/// All packages go through yay, which handles both official and AUR.
pub fn resolve(config: &BlunuxConfig) -> Vec<String> {
    let mut pkgs: Vec<String> = Vec::new();
    let p = &config.packages;

    // Desktop
    if p.desktop.kde {
        pkgs.extend(
            [
                "plasma-desktop",
                "plasma-workspace",
                "sddm",
                "konsole",
                "dolphin",
                "kate",
                "ark",
                "spectacle",
                "xdg-desktop-portal-kde",
            ]
            .map(str::to_string),
        );
    }

    // Browsers
    if p.browser.firefox {
        pkgs.push("firefox".into());
    }
    if p.browser.whale {
        pkgs.push("naver-whale-bin".into());
    }
    if p.browser.chrome {
        pkgs.push("google-chrome".into());
    }
    if p.browser.mullvad {
        pkgs.push("mullvad-browser-bin".into());
    }

    // Office
    if p.office.libreoffice {
        pkgs.push("libreoffice-fresh".into());
    }
    if p.office.hoffice {
        pkgs.push("hoffice-bin".into());
    }
    if p.office.texlive {
        pkgs.extend(["texlive-core", "texlive-latexextra"].map(str::to_string));
    }

    // Development
    if p.development.vscode {
        pkgs.push("visual-studio-code-bin".into());
    }
    if p.development.sublime {
        pkgs.push("sublime-text-4".into());
    }
    if p.development.rust {
        pkgs.push("rustup".into());
    }
    if p.development.julia {
        pkgs.push("julia".into());
    }
    if p.development.nodejs {
        pkgs.extend(["nodejs", "npm"].map(str::to_string));
    }
    if p.development.github_cli {
        pkgs.push("github-cli".into());
    }

    // Multimedia
    if p.multimedia.obs {
        pkgs.push("obs-studio".into());
    }
    if p.multimedia.vlc {
        pkgs.push("vlc".into());
    }
    if p.multimedia.freetv {
        pkgs.push("freetuxtv".into());
    }
    if p.multimedia.ytdlp {
        pkgs.push("yt-dlp".into());
    }
    if p.multimedia.freetube {
        pkgs.push("freetube-bin".into());
    }

    // Gaming
    if p.gaming.steam {
        pkgs.extend(["steam", "lib32-mesa", "lib32-vulkan-radeon"].map(str::to_string));
    }
    if p.gaming.unciv {
        pkgs.push("unciv-bin".into());
    }
    if p.gaming.snes9x {
        pkgs.push("snes9x-gtk".into());
    }

    // Virtualization
    if p.virtualization.virtualbox {
        pkgs.extend(["virtualbox", "virtualbox-host-dkms"].map(str::to_string));
    }
    if p.virtualization.docker {
        pkgs.extend(["docker", "docker-compose"].map(str::to_string));
    }

    // Communication
    if p.communication.teams {
        pkgs.push("teams-for-linux-bin".into());
    }
    if p.communication.whatsapp {
        pkgs.push("whatsapp-for-linux".into());
    }
    if p.communication.onenote {
        pkgs.push("p3x-onenote-bin".into());
    }

    // Utility
    if p.utility.conky {
        pkgs.push("conky".into());
    }
    if p.utility.vnc {
        pkgs.push("tigervnc".into());
    }
    if p.utility.samba {
        pkgs.extend(["samba", "smbclient"].map(str::to_string));
    }
    if p.utility.bluetooth {
        pkgs.extend(["bluez", "bluez-utils", "bluedevil"].map(str::to_string));
    }

    // Kernel
    let kernel = &config.kernel.kernel_type;
    if kernel != "linux" {
        pkgs.push(kernel.clone());
        pkgs.push(format!("{kernel}-headers"));
    }

    pkgs
}
