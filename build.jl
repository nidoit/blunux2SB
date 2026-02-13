#!/usr/bin/env julia
#
# blunux2 ISO Build Orchestrator
#
# Reads config.toml, generates the archiso profile, builds Rust binaries,
# and creates the Live ISO via mkarchiso.
#
# Usage:
#   julia build.jl                    # Full build
#   julia build.jl --profile-only     # Generate profile without building ISO
#   julia build.jl --skip-rust        # Skip cargo build (use existing binaries)

using TOML

const ROOT       = @__DIR__
const CONFIG     = joinpath(ROOT, "config.toml")
const PROFILE    = joinpath(ROOT, "profile")
const WORK_DIR   = get(ENV, "BLUNUX_WORK", "/tmp/blunux2-work")
const OUT_DIR    = get(ENV, "BLUNUX_OUT", joinpath(ROOT, "out"))

# ---------------------------------------------------------------------------
# Config loading
# ---------------------------------------------------------------------------

function load_config()
    if !isfile(CONFIG)
        error("config.toml not found at $CONFIG")
    end
    println("── Loading config.toml ──")
    cfg = TOML.parsefile(CONFIG)
    println("  Name: $(cfg["blunux"]["name"])")
    println("  Version: $(cfg["blunux"]["version"])")
    return cfg
end

# ---------------------------------------------------------------------------
# Package list generation
# ---------------------------------------------------------------------------

function generate_packages(cfg::Dict)
    println("\n── Generating packages.x86_64 ──")

    pkgs = String[]   # Official repo packages
    aur  = String[]   # AUR/custom repo packages (require custom repo or pre-built)

    # Base system (always included)
    append!(pkgs, [
        "base", "linux", "linux-firmware", "linux-headers",
        "mkinitcpio", "mkinitcpio-archiso",
    ])

    # Kernel override
    kernel = get(get(cfg, "kernel", Dict()), "type", "linux")
    if kernel != "linux"
        push!(pkgs, kernel)
        push!(pkgs, "$(kernel)-headers")
    end

    # Boot
    bootloader = get(get(cfg, "install", Dict()), "bootloader", "grub")
    append!(pkgs, ["efibootmgr"])
    if bootloader == "grub"
        append!(pkgs, ["grub", "syslinux"])
    elseif bootloader == "systemd-boot"
        push!(pkgs, "syslinux")
    else  # nmbl (EFISTUB)
        push!(pkgs, "syslinux")
    end

    # Filesystem
    append!(pkgs, ["btrfs-progs", "dosfstools", "ntfs-3g", "e2fsprogs"])

    # Network
    append!(pkgs, ["networkmanager", "iwd", "openssh"])

    # Display & audio
    append!(pkgs, [
        "xorg-server", "xorg-xinit", "wayland", "pipewire",
        "pipewire-pulse", "wireplumber",
    ])

    # Drivers (base — auto-detection adds more at runtime)
    append!(pkgs, ["mesa", "vulkan-radeon", "vulkan-intel",
                    "nvidia-dkms", "nvidia-utils"])

    # Fonts
    append!(pkgs, ["noto-fonts", "noto-fonts-cjk", "noto-fonts-emoji", "ttf-liberation"])

    # Installer (AUR — must be pre-built into custom repo)
    append!(aur, ["calamares", "calamares-extensions"])

    # Desktop environment
    packages = get(cfg, "packages", Dict())
    desktop = get(packages, "desktop", Dict())
    if get(desktop, "kde", false)
        append!(pkgs, [
            "plasma-desktop", "plasma-workspace", "sddm",
            "kde-applications-meta", "xdg-desktop-portal-kde",
        ])
    end

    # Browsers
    browser = get(packages, "browser", Dict())
    get(browser, "firefox", false) && push!(pkgs, "firefox")

    # Office
    office = get(packages, "office", Dict())
    get(office, "libreoffice", false) && push!(pkgs, "libreoffice-fresh")

    # Development
    dev = get(packages, "development", Dict())
    get(dev, "nodejs", false)    && append!(pkgs, ["nodejs", "npm"])
    get(dev, "github_cli", false) && push!(pkgs, "github-cli")

    # Essential apps
    append!(pkgs, ["konsole", "dolphin", "kate", "git", "base-devel"])

    # Input method
    im = get(cfg, "input_method", Dict())
    if get(im, "enabled", false)
        engine = get(im, "engine", "kime")
        if engine == "kime"
            append!(aur, ["kime", "kime-indicator"])
        elseif engine == "fcitx5"
            append!(pkgs, ["fcitx5", "fcitx5-gtk", "fcitx5-qt", "fcitx5-configtool"])
            push!(aur, "fcitx5-hangul")
        elseif engine == "ibus"
            push!(pkgs, "ibus")
            push!(aur, "ibus-hangul")
        end
    end

    # Bluetooth
    utility = get(packages, "utility", Dict())
    if get(utility, "bluetooth", false)
        append!(pkgs, ["bluez", "bluez-utils", "bluedevil"])
    end

    # Note: blunux2-settings, blunux2-themes, blunux2-calamares-config
    # are not packaged yet. blunux-setup handles configuration at runtime
    # via config.toml instead of distro packages.

    # Check if custom repo is enabled in pacman.conf
    pacman_conf = read(joinpath(PROFILE, "pacman.conf"), String)
    custom_repo_enabled = occursin(r"^\[blunux2\]"m, pacman_conf)

    # Write packages.x86_64
    pkg_file = joinpath(PROFILE, "packages.x86_64")
    open(pkg_file, "w") do f
        println(f, "# Official repo packages")
        for pkg in unique(pkgs)
            println(f, pkg)
        end
        if custom_repo_enabled
            println(f, "\n# AUR/custom repo packages")
            for pkg in unique(aur)
                println(f, pkg)
            end
        else
            println(f, "\n# AUR/custom repo packages (commented out — enable [blunux2] repo in pacman.conf)")
            for pkg in unique(aur)
                println(f, "#$pkg")
            end
        end
    end

    println("  Wrote $(length(unique(pkgs))) official packages")
    if !isempty(aur)
        if custom_repo_enabled
            println("  Wrote $(length(unique(aur))) custom repo packages")
        else
            println("  Skipped $(length(unique(aur))) AUR/custom packages (no custom repo)")
            println("  ⚠  To include them, enable [blunux2] repo in profile/pacman.conf")
        end
    end
end

# ---------------------------------------------------------------------------
# Airootfs overlay
# ---------------------------------------------------------------------------

function generate_airootfs(cfg::Dict)
    println("\n── Generating airootfs overlay ──")

    locale = get(cfg, "locale", Dict())
    install = get(cfg, "install", Dict())

    # Create required directories
    for dir in [
        "airootfs/etc/mkinitcpio.conf.d",
        "airootfs/usr/share/blunux",
        "airootfs/usr/bin",
    ]
        mkpath(joinpath(PROFILE, dir))
    end

    # hostname
    write(joinpath(PROFILE, "airootfs/etc/hostname"), get(install, "hostname", "blunux") * "\n")

    # locale.conf
    lang = get(locale, "language", ["en_US"])[1]
    write(joinpath(PROFILE, "airootfs/etc/locale.conf"), "LANG=$(lang).UTF-8\n")

    # vconsole.conf
    kb = get(locale, "keyboard", ["us"])[1]
    write(joinpath(PROFILE, "airootfs/etc/vconsole.conf"), "KEYMAP=$(kb)\n")

    # mkinitcpio archiso hooks
    write(joinpath(PROFILE, "airootfs/etc/mkinitcpio.conf.d/archiso.conf"),
        """HOOKS=(base udev microcode modconf kms memdisk archiso archiso_loop_mnt archiso_pxe_common archiso_pxe_nbd archiso_pxe_http archiso_pxe_nfs block filesystems keyboard)\n""")

    # Copy config.toml into the ISO
    cp(CONFIG, joinpath(PROFILE, "airootfs/usr/share/blunux/config.toml"), force=true)

    println("  Generated hostname, locale.conf, vconsole.conf, mkinitcpio hooks")
    println("  Copied config.toml into airootfs")
end

# ---------------------------------------------------------------------------
# Rust build
# ---------------------------------------------------------------------------

function build_rust()
    println("\n── Building Rust binaries ──")

    cmd = `cargo build --release --manifest-path $(joinpath(ROOT, "Cargo.toml"))`
    println("  Running: $cmd")
    run(cmd)

    target = joinpath(ROOT, "target/release")
    bindir = joinpath(PROFILE, "airootfs/usr/bin")

    for bin in ["blunux-wizard", "blunux-toml2cal", "blunux-setup"]
        src = joinpath(target, bin)
        dst = joinpath(bindir, bin)
        if isfile(src)
            cp(src, dst, force=true)
            chmod(dst, 0o755)
            println("  Installed $bin → airootfs/usr/bin/")
        else
            @warn "Binary not found: $src"
        end
    end

    # Copy shell scripts
    scriptsdir = joinpath(ROOT, "scripts")
    for script in ["startblunux", "calamares-blunux"]
        src = joinpath(scriptsdir, script)
        dst = joinpath(bindir, script)
        if isfile(src)
            cp(src, dst, force=true)
            chmod(dst, 0o755)
            println("  Installed $script → airootfs/usr/bin/")
        end
    end
end

# ---------------------------------------------------------------------------
# ISO build
# ---------------------------------------------------------------------------

function build_iso()
    println("\n── Building ISO with mkarchiso ──")

    mkdirs = [WORK_DIR, OUT_DIR]
    for d in mkdirs
        mkpath(d)
    end

    cmd = `sudo mkarchiso -v -w $WORK_DIR -o $OUT_DIR $PROFILE`
    println("  Running: $cmd")
    println("  Work dir: $WORK_DIR")
    println("  Output dir: $OUT_DIR")
    run(cmd)

    # Find the generated ISO
    isos = filter(f -> endswith(f, ".iso"), readdir(OUT_DIR))
    if !isempty(isos)
        println("\n  ✓ ISO created: $(joinpath(OUT_DIR, isos[end]))")
    end
end

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

function main()
    println("╔══════════════════════════════════════╗")
    println("║     blunux2 ISO Builder v2.0         ║")
    println("║     Julia build → Rust + C ISO       ║")
    println("╚══════════════════════════════════════╝")
    println()

    args = ARGS

    profile_only = "--profile-only" in args
    skip_rust    = "--skip-rust" in args

    # 1. Load config
    cfg = load_config()

    # 2. Generate archiso profile
    generate_packages(cfg)
    generate_airootfs(cfg)

    # 3. Build Rust binaries
    if !skip_rust
        build_rust()
    else
        println("\n── Skipping Rust build (--skip-rust) ──")
    end

    # 4. Build ISO
    if !profile_only
        build_iso()
    else
        println("\n── Profile generated. Skipping ISO build (--profile-only) ──")
        println("  To build manually: sudo mkarchiso -v -w $WORK_DIR -o $OUT_DIR $PROFILE")
    end
end

main()
