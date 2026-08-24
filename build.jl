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
        "mkinitcpio-nfs-utils",  # ipconfig + nfsmount for PXE hooks
        "nbd",                   # nbd-client for archiso_pxe_nbd hook
        "pv",                    # progress viewer for copy-to-RAM
    ])

    # Kernel override
    kernel = get(get(cfg, "kernel", Dict()), "type", "linux")
    if kernel != "linux"
        push!(pkgs, kernel)
        push!(pkgs, "$(kernel)-headers")
    end

    # Boot — the live ISO always uses GRUB; the user's bootloader choice
    # in config.toml only applies to the installed system.
    append!(pkgs, ["efibootmgr", "grub", "syslinux"])

    # Filesystem
    append!(pkgs, ["dosfstools", "ntfs-3g", "e2fsprogs"])

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

    generate_live_session()
end

"""
    generate_live_session()

Wire up the unattended live session: boot → desktop → installer, with no
login prompt and no terminal.

Without this the ISO boots to a getty login prompt, because nothing creates
a live user, nothing enables autologin, and nothing launches the installer
once the desktop is up. `startblunux` documents that it is "called by SDDM
auto-login", but that autologin never existed.

The session runs as an unprivileged `liveuser` rather than root: Plasma
refuses to run as root, and toml2cal already expects /home/liveuser when it
copies the live theme onto the installed system.
"""
function generate_live_session()
    println("\n── Generating live session (autologin → installer) ──")

    a(p) = joinpath(PROFILE, "airootfs", p)
    for dir in [
        "etc/sysusers.d", "etc/tmpfiles.d", "etc/sudoers.d",
        "etc/systemd/system/getty@tty1.service.d",
        "etc/systemd/system",
        "home/liveuser/.config/autostart",
    ]
        mkpath(a(dir))
    end

    # Live user, created at boot by systemd-sysusers. Declaring it this way
    # avoids shipping an /etc/passwd that would clobber the system users
    # pacstrap created (sddm, polkitd, …).
    write(a("etc/sysusers.d/blunux-live.conf"), """
        # blunux2 live session user
        u liveuser 1000 "Blunux Live" /home/liveuser /bin/bash
        m liveuser wheel
        """)

    # The home directory ships in the ISO (see below) — tmpfiles only has to
    # take ownership, since the squashfs is built as root.
    write(a("etc/tmpfiles.d/blunux-live.conf"), """
        # blunux2 live session home
        z /home/liveuser 0755 liveuser liveuser - -
        Z /home/liveuser/.config 0755 liveuser liveuser - -
        """)

    # sysusers creates the account with a locked password, which would block
    # autologin. Clear it before any login happens.
    write(a("etc/systemd/system/blunux-live-user.service"), """
        [Unit]
        Description=Prepare blunux live user
        After=systemd-sysusers.service systemd-tmpfiles-setup.service
        Before=getty@tty1.service display-manager.service

        [Service]
        Type=oneshot
        RemainAfterExit=yes
        ExecStart=/usr/bin/passwd -d liveuser
        ExecStart=/usr/bin/chown -R liveuser:liveuser /home/liveuser

        [Install]
        WantedBy=multi-user.target
        """)

    # The installer and blunux-setup both need root; a live medium has no
    # password to type, so grant it outright.
    write(a("etc/sudoers.d/blunux-live"), "liveuser ALL=(ALL:ALL) NOPASSWD: ALL\n")

    # Autologin on tty1. liveuser's shell profile takes it from here.
    write(a("etc/systemd/system/getty@tty1.service.d/autologin.conf"), """
        [Service]
        ExecStart=
        ExecStart=-/usr/bin/agetty --noreset --noclear --autologin liveuser - \${TERM}
        """)

    # Shell profile: start the graphical live session on tty1 only, so a
    # second console stays usable for troubleshooting. The /run/archiso guard
    # matters because this file is inside the squashfs that gets unpacked onto
    # the installed system — without it, logging into the installed machine on
    # tty1 would re-run the live setup.
    write(a("home/liveuser/.bash_profile"), """
        # blunux2 live session
        if [[ -d /run/archiso && -z \${DISPLAY:-} && -z \${WAYLAND_DISPLAY:-} \\
              && \$(tty) == /dev/tty1 ]]; then
            exec startblunux
        fi
        """)

    # Once Plasma is up, open the installer automatically. It is an ordinary
    # window — closing it leaves a usable live desktop. Kept in liveuser's home
    # rather than /etc/skel so users created by the installer don't inherit it.
    write(a("home/liveuser/.config/autostart/blunux-installer.desktop"), """
        [Desktop Entry]
        Type=Application
        Name=Install Blunux
        Name[ko]=Blunux 설치
        Comment=Install Blunux to this computer
        Comment[ko]=이 컴퓨터에 Blunux를 설치합니다
        Exec=calamares-blunux
        Icon=system-software-install
        Terminal=false
        X-GNOME-Autostart-enabled=true
        """)

    # Also leave a launcher in the menu/desktop for a second run.
    mkpath(a("usr/share/applications"))
    write(a("usr/share/applications/blunux-installer.desktop"), """
        [Desktop Entry]
        Type=Application
        Name=Install Blunux
        Name[ko]=Blunux 설치
        Comment=Install Blunux to this computer
        Comment[ko]=이 컴퓨터에 Blunux를 설치합니다
        Exec=calamares-blunux
        Icon=system-software-install
        Terminal=false
        Categories=System;
        """)

    # Enable the live-user unit the way systemd would (mkarchiso ships the
    # airootfs as-is; there is no systemctl run against it).
    wants = a("etc/systemd/system/multi-user.target.wants")
    mkpath(wants)
    link = joinpath(wants, "blunux-live-user.service")
    islink(link) || symlink("/etc/systemd/system/blunux-live-user.service", link)

    println("  liveuser + autologin on tty1")
    println("  startblunux from shell profile → Plasma")
    println("  Calamares autostarts in the desktop session")
end

# ---------------------------------------------------------------------------
# Rust build
# ---------------------------------------------------------------------------

function build_rust(cfg::Dict)
    println("\n── Building Rust binaries ──")

    cmd = `cargo build --release --manifest-path $(joinpath(ROOT, "Cargo.toml"))`
    println("  Running: $cmd")
    run(cmd)

    target = joinpath(ROOT, "target/release")
    bindir = joinpath(PROFILE, "airootfs/usr/bin")
    sharedir = joinpath(PROFILE, "airootfs/usr/share/blunux")
    cardsdir = joinpath(PROFILE, "airootfs/usr/share/blunux-installer/cards")

    # Conditionally include ai-agent
    # Check both [packages.ai].agent (ISO build flag) and [ai_agent].enabled (runtime flag)
    ai_pkg = get(get(get(cfg, "packages", Dict()), "ai", Dict()), "agent", false)
    ai_rt  = get(get(cfg, "ai_agent", Dict()), "enabled", false)
    ai = ai_pkg || ai_rt

    binaries_to_copy = ["blunux-wizard", "blunux-toml2cal", "blunux-setup"]
    if ai
        println("  ai enabled → including blunux-ai + installer assets")
        push!(binaries_to_copy, "blunux-ai")
    else
        println("  ai disabled → skipping blunux-ai")
    end

    for bin in binaries_to_copy
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

    # Copy AI Agent installer assets into the ISO
    if ai
        mkpath(sharedir)
        mkpath(cardsdir)

        install_script = joinpath(ROOT, "blunux-ai-installer", "install-ai-agent.sh")
        card_json      = joinpath(ROOT, "blunux-ai-installer", "ai-agent.card.json")

        if isfile(install_script)
            dst = joinpath(sharedir, "install-ai-agent.sh")
            cp(install_script, dst, force=true)
            chmod(dst, 0o755)
            println("  Installed install-ai-agent.sh → airootfs/usr/share/blunux/")
        else
            @warn "install-ai-agent.sh not found: $install_script"
        end

        if isfile(card_json)
            dst = joinpath(cardsdir, "ai-agent.card.json")
            cp(card_json, dst, force=true)
            println("  Installed ai-agent.card.json → airootfs/usr/share/blunux-installer/cards/")
        else
            @warn "ai-agent.card.json not found: $card_json"
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

    # mkarchiso is resumable: it drops a sentinel file per completed stage in
    # the work dir and skips those stages on the next run. A leftover work dir
    # therefore makes it skip *everything* — including ISO creation — and exit
    # 0 having built nothing. Always start from a clean work dir.
    if isdir(WORK_DIR) && !isempty(readdir(WORK_DIR))
        println("  Clearing stale work dir: $WORK_DIR")
        run(`sudo rm -rf $WORK_DIR`)
    end

    for d in [WORK_DIR, OUT_DIR]
        mkpath(d)
    end

    started_at = time()

    cmd = `sudo mkarchiso -v -w $WORK_DIR -o $OUT_DIR $PROFILE`
    println("  Running: $cmd")
    println("  Work dir: $WORK_DIR")
    println("  Output dir: $OUT_DIR")
    run(cmd)

    # Only count ISOs this run actually produced — listing whatever happens to
    # sit in OUT_DIR would report a leftover ISO from an earlier build as a
    # success and hide a failure like the one above.
    fresh = filter(readdir(OUT_DIR)) do f
        endswith(f, ".iso") && mtime(joinpath(OUT_DIR, f)) >= started_at
    end

    if isempty(fresh)
        error("""
              mkarchiso exited successfully but produced no ISO in $OUT_DIR.
              This usually means a stale work dir made it skip every stage.
              Remove it and retry:  sudo rm -rf $WORK_DIR
              """)
    end

    iso_path = joinpath(OUT_DIR, first(sort(fresh)))
    size_gb = round(filesize(iso_path) / 1024^3, digits = 2)
    println("\n  ✓ ISO created: $iso_path ($(size_gb) GB)")

    verify_boot_label(iso_path)
end

"""
    verify_boot_label(iso_path)

Confirm the boot configs inside the ISO search for the label the ISO
actually carries.

The boot configs use mkarchiso's `%ARCHISO_LABEL%` placeholder, which is
substituted with `iso_label` from profiledef.sh. Writing a prefix in front
of the placeholder (`archisolabel=BLUNUX2_%ARCHISO_LABEL%` when iso_label
is already `BLUNUX2_YYYYMM`) produces a doubled prefix, and the resulting
ISO boots to "device did not show up" because initramfs looks for a label
that does not exist. Nothing about the ISO looks wrong until you try to
boot it, so check it here.
"""
function verify_boot_label(iso_path)
    print("  Verifying boot label ... ")

    actual = try
        strip(read(`blkid -o value -s LABEL $iso_path`, String))
    catch
        println("skipped (blkid unavailable)")
        return
    end

    tmp = mktempdir()
    try
        cfg = joinpath(tmp, "syslinux.cfg")
        try
            run(pipeline(
                `xorriso -osirrox on -indev $iso_path
                         -extract /boot/syslinux/syslinux.cfg $cfg`,
                stdout = devnull, stderr = devnull,
            ))
        catch
            println("skipped (could not extract boot config)")
            return
        end

        wanted = unique([m.captures[1] for m in
                         eachmatch(r"archisolabel=(\S+)", read(cfg, String))])

        if isempty(wanted)
            println("skipped (no archisolabel in boot config)")
            return
        end

        bad = filter(!=(actual), wanted)
        if !isempty(bad)
            error("""
                  Boot config label does not match the ISO volume label.
                  ISO volume label : $actual
                  Boot config wants: $(join(bad, ", "))
                  This ISO will fail to boot ("device did not show up").
                  Check archisolabel= in profile/syslinux/syslinux.cfg,
                  profile/efiboot/loader/entries/*.conf and profile/grub/grub.cfg —
                  use a bare %ARCHISO_LABEL% with no prefix of its own, since
                  iso_label in profiledef.sh already carries one.
                  """)
        end

        println("ok ($actual)")
    finally
        rm(tmp; recursive = true, force = true)
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
        build_rust(cfg)
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
