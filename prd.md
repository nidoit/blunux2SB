# PRD: blunux2 — Custom Arch-Based Linux Distribution

**Version:** 1.0  
**Author:** Jaewoo Joung  
**Date:** 2026-02-13  
**Status:** Draft

---

## 1. Overview

**blunux2** is a custom Linux distribution built on Arch Linux (or Manjaro as a stabilized Arch base). It provides a polished Live OS experience with a graphical installer, pre-configured desktop environment, and curated software stack — targeting users who want an Arch-based system without manual setup complexity.

This PRD documents the complete technical architecture of how a Live ISO is built, how the Live OS boots, and how the system is installed to disk.

---

## 2. Architecture Overview

The system consists of four major subsystems:

```
┌─────────────────────────────────────────────────────┐
│                  blunux2 Distribution                │
├──────────┬──────────┬───────────┬───────────────────┤
│ ISO Build│ Live Boot│ Setup     │ Disk Installation │
│ System   │ System   │ Wizard    │ (Calamares)       │
├──────────┼──────────┼───────────┼───────────────────┤
│ Julia    │ GRUB/    │ Rust      │ config.toml →     │
│ build.jl │ syslinux │ wizard    │ Calamares YAML    │
│ (dev-side│ initramfs│ binary    │ (auto-translated) │
│ only, not│ squashfs │ C fallback│ ext4 default      │
│ in ISO)  │ overlayfs│           │                   │
└──────────┴──────────┴───────────┴───────────────────┘
     ↑ Developer machine               ↑ Inside the ISO
```

**Key distinction:** Julia is used only on the **developer's machine** to orchestrate the ISO build process (`build.jl`). It is NOT shipped inside the ISO. The ISO contains only Rust binaries + C fallback.

---

## 3. Subsystem 1: ISO Build System

### 3.1 Foundation — archiso

The ISO is built using **archiso**, the official Arch Linux ISO creation tool. archiso uses a **profile** directory that defines everything about the ISO.

#### Profile Directory Structure

```
blunux2-profile/
├── profiledef.sh              # Master build configuration
├── packages.x86_64            # Package list (one per line)
├── pacman.conf                # Package manager config (repos, mirrors)
├── airootfs/                  # Root filesystem overlay
│   ├── etc/
│   │   ├── mkinitcpio.conf.d/
│   │   │   └── archiso.conf   # initramfs hooks config
│   │   ├── mkinitcpio.d/
│   │   │   └── linux.preset   # Kernel preset for initramfs
│   │   ├── systemd/system/    # Systemd service enablement
│   │   ├── skel/              # Default user skeleton files
│   │   ├── pacman.conf        # Target system pacman.conf
│   │   ├── hostname
│   │   ├── locale.conf
│   │   └── vconsole.conf
│   ├── root/                  # Root user home in live session
│   └── usr/
│       ├── bin/
│       │   ├── startblunux          # Main live session entry point
│       │   ├── blunux-wizard        # Setup wizard (Rust binary — hw detect, config, desktop launch)
│       │   ├── calamares-blunux     # Installer wrapper script
│       │   └── blunux-toml2cal      # config.toml → Calamares translator (Rust)
│       ├── share/blunux/
│       │   ├── livecd/              # Setup wizard assets
│       │   ├── calamares/           # Installer config templates
│       │   └── config.toml          # User-facing install configuration
│       └── lib/calamares/           # Custom Calamares modules
├── efiboot/                   # UEFI boot configuration
│   └── loader/
│       ├── loader.conf
│       └── entries/
│           └── blunux.conf
├── syslinux/                  # BIOS boot configuration
│   ├── syslinux.cfg
│   └── splash.png
└── grub/                      # GRUB configuration (UEFI)
    └── grub.cfg
```

### 3.2 profiledef.sh — Master Configuration

```bash
#!/usr/bin/env bash

iso_name="blunux2"
iso_label="BLUNUX2_$(date +%Y%m)"
iso_publisher="blunux2 Project <https://blunux2.dev>"
iso_application="blunux2 Live/Install Medium"
iso_version="$(date +%Y.%m.%d)"
install_dir="arch"
buildmodes=('iso')
bootmodes=(
    'bios.syslinux.mbr'      # Legacy BIOS from MBR
    'bios.syslinux.eltorito'  # Legacy BIOS from optical
    'uefi-x64.grub.esp'       # 64-bit UEFI
    'uefi-x64.grub.eltorito'  # 64-bit UEFI optical
)
arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"           # or "erofs" for baseline
airootfs_image_tool_options=(
    '-comp' 'zstd' '-Xcompression-level' '15'  # High compression
    '-b' '1M'                                    # 1MB block size
)
file_permissions=(
    ["/etc/shadow"]="0:0:400"
    ["/usr/bin/startblunux"]="0:0:755"
    ["/usr/bin/calamares-blunux"]="0:0:755"
    ["/usr/bin/blunux-toml2cal"]="0:0:755"
)
```

### 3.3 packages.x86_64 — Package Selection

```
# ── Base System ──
base
linux
linux-firmware
linux-headers
mkinitcpio
mkinitcpio-archiso

# ── Boot ──
grub
efibootmgr
syslinux

# ── Filesystem ──
dosfstools
ntfs-3g
e2fsprogs

# ── Network ──
networkmanager
iwd
openssh

# ── Desktop Environment ──
plasma-desktop
plasma-workspace
sddm
kde-applications-meta    # or selective subset

# ── Display ──
xorg-server
xorg-xinit
wayland
xdg-desktop-portal-kde

# ── Drivers ──
mesa
vulkan-radeon
vulkan-intel
nvidia-dkms              # Proprietary NVIDIA
nvidia-utils

# ── Audio ──
pipewire
pipewire-pulse
wireplumber

# ── Fonts ──
noto-fonts
noto-fonts-cjk
noto-fonts-emoji
ttf-liberation

# ── Installer ──
calamares
calamares-extensions

# ── Build Toolchain (Rust + C) ──
rust                     # Wizard, config translator, hw detect, UI
gcc                      # C compiler for low-level fallback code
cmake                    # Build system for C components
gtk4                     # GTK4 UI library (used via Rust gtk4-rs)
libadwaita               # Libadwaita for modern GNOME-style widgets

# ── Essential Apps ──
firefox
libreoffice-fresh
konsole
dolphin
kate
git
base-devel

# ── blunux2 Custom ──
# (from custom repo or AUR)
blunux2-settings
blunux2-livecd           # Setup wizard (Rust binary, no Python)
blunux2-themes
blunux2-calamares-config
blunux2-toml2cal         # config.toml → Calamares YAML translator (Rust)
```

### 3.4 Build Process

The ISO build is orchestrated by **Julia** (`build.jl`) on the developer's machine. Julia reads `config.toml`, generates the archiso profile, builds the Rust binaries, and calls `mkarchiso`.

#### Local Build (Julia orchestrator)

```bash
# Prerequisites: archiso, Julia, Rust
sudo pacman -S archiso

# Full build — config.toml → profile → Rust binaries → ISO
julia build.jl

# Profile only (no ISO build, for inspection)
julia build.jl --profile-only

# Skip Rust build (use existing binaries)
julia build.jl --skip-rust

# Output: out/blunux2-YYYY.MM.DD-x86_64.iso
```

#### What build.jl Does

1. **Reads `config.toml`** — Parses user configuration
2. **Generates `packages.x86_64`** — Maps config booleans to pacman package names
3. **Generates airootfs overlay** — hostname, locale.conf, vconsole.conf, copies config.toml
4. **Builds Rust binaries** — `cargo build --release` → copies `blunux-wizard` and `blunux-toml2cal` into airootfs
5. **Calls `mkarchiso`** — Standard archiso ISO build from the generated profile

#### Manual Build (without Julia)

```bash
# If you prefer not to use Julia, you can build manually:
cargo build --release
cp target/release/blunux-{wizard,toml2cal} profile/airootfs/usr/bin/
cp scripts/{startblunux,calamares-blunux} profile/airootfs/usr/bin/
sudo mkarchiso -v -w /tmp/blunux2-work -o out/ profile/
```

#### What mkarchiso Does Internally

1. **pacstrap** — Installs all packages from `packages.x86_64` into a working rootfs directory
2. **Overlay airootfs/** — Copies custom configs over the installed rootfs
3. **Run mkinitcpio** — Generates initramfs with archiso hooks inside the rootfs
4. **Create SquashFS** — Compresses the entire rootfs into `airootfs.sfs` (~2-4GB → ~1.5-2.5GB)
5. **Build ISO image** — Assembles bootloader configs, kernel, initramfs, and squashfs into a hybrid ISO (bootable as both USB and optical)

#### CI/CD Build (GitHub Actions)

```yaml
name: blunux2_iso_build
on:
  workflow_dispatch:
  schedule:
    - cron: '0 6 1 * *'  # Monthly builds

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build ISO
        uses: manjaro/manjaro-iso-action@main  # or custom action
        with:
          edition: kde
          branch: stable
          scope: full

      - name: Upload ISO
        uses: actions/upload-artifact@v4
        with:
          name: blunux2-iso
          path: "*.iso"

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          files: |
            *.iso
            *.iso.md5
            *.iso.torrent
```

---

## 4. Subsystem 2: Live Boot Process

### 4.1 Boot Flow (Power-On to Desktop)

```
┌──────────┐    ┌───────────┐    ┌──────────────┐    ┌────────────┐
│ Firmware  │───▶│ Bootloader│───▶│  initramfs   │───▶│  systemd   │
│ BIOS/UEFI│    │ GRUB/     │    │  (archiso    │    │  (PID 1)   │
│           │    │ syslinux  │    │   hooks)     │    │            │
└──────────┘    └───────────┘    └──────────────┘    └────────────┘
                                        │                    │
                                        ▼                    ▼
                                 ┌──────────────┐    ┌────────────┐
                                 │ Mount        │    │ SDDM /     │
                                 │ squashfs +   │    │ startblunux│
                                 │ overlayfs    │    │ wizard     │
                                 └──────────────┘    └────────────┘
                                                          │
                                                          ▼
                                                   ┌────────────┐
                                                   │  KDE Plasma│
                                                   │  Desktop   │
                                                   └────────────┘
```

### 4.2 Stage 1: Firmware → Bootloader

**UEFI Path:**
1. Firmware reads ESP (EFI System Partition) on ISO/USB
2. Loads `EFI/BOOT/BOOTx64.EFI` (GRUB)
3. GRUB reads `grub.cfg`

**BIOS Path:**
1. Firmware reads MBR → loads syslinux
2. syslinux reads `syslinux.cfg`

#### GRUB Boot Entry (grub.cfg)

```
menuentry "blunux2 Live (default)" {
    set gfxpayload=keep
    linux /arch/boot/x86_64/vmlinuz-linux
        archisobasedir=arch
        archisolabel=BLUNUX2_202602
        cow_spacesize=4G
        driver=nonfree
        quiet splash
    initrd /arch/boot/x86_64/initramfs-linux.img
}

menuentry "blunux2 Live (open-source drivers)" {
    linux /arch/boot/x86_64/vmlinuz-linux
        archisobasedir=arch
        archisolabel=BLUNUX2_202602
        cow_spacesize=4G
        driver=free
        quiet splash
    initrd /arch/boot/x86_64/initramfs-linux.img
}

menuentry "blunux2 Live (to RAM)" {
    linux /arch/boot/x86_64/vmlinuz-linux
        archisobasedir=arch
        archisolabel=BLUNUX2_202602
        copytoram
        quiet splash
    initrd /arch/boot/x86_64/initramfs-linux.img
}
```

**Key kernel parameters:**
- `archisobasedir=arch` — Where to find the squashfs on the media
- `archisolabel=BLUNUX2_202602` — Volume label to identify the boot device
- `cow_spacesize=4G` — Size of the writable overlay in RAM
- `copytoram` — Copy entire squashfs to RAM (allows removing USB after boot)

### 4.3 Stage 2: initramfs — The Magic of Live Booting

The initramfs is a small temporary root filesystem loaded into RAM. It contains just enough to find and mount the real root.

#### mkinitcpio Hooks (archiso.conf)

```bash
HOOKS=(base udev microcode modconf kms memdisk archiso archiso_loop_mnt
       archiso_pxe_common archiso_pxe_nbd archiso_pxe_http archiso_pxe_nfs
       block filesystems keyboard)
```

**Critical hook: `archiso`** — This is where the live boot magic happens:

1. **Find the boot device** — Scans for a device with label `BLUNUX2_202602`
2. **Mount the boot device** — Mounts the ISO filesystem (ISO 9660 or FAT32)
3. **Locate squashfs** — Finds `/arch/x86_64/airootfs.sfs`
4. **Mount squashfs** — Mounts the compressed root image as read-only
5. **Create overlayfs** — Sets up a layered filesystem:

```
┌────────────────────────────────┐
│        overlayfs (merged)      │  ← What the user sees as /
├────────────────────────────────┤
│   upperdir (tmpfs in RAM)      │  ← All writes go here (volatile)
├────────────────────────────────┤
│   lowerdir (squashfs, ro)      │  ← The compressed root image
└────────────────────────────────┘
```

This is the fundamental trick: the squashfs provides a complete read-only root filesystem, and overlayfs layers a writable tmpfs on top. Any file modifications during the live session are stored only in RAM and lost on reboot.

### 4.4 Stage 3: systemd Init → Desktop

Once the overlayfs root is assembled and `switch_root` is called:

1. **systemd (PID 1)** starts and reads unit files
2. **Basic services** — networking, audio, etc.
3. **Display manager (SDDM)** starts
4. **Auto-login** — Live session logs in automatically as `liveuser`
5. **startblunux script** runs — launches the setup wizard

### 4.5 Setup Wizard (First-Run Experience)

The setup wizard is a single **Rust binary** (`blunux-wizard`) — no Python, no Julia, no intermediate layers:

- **Rust** — All logic: hardware detection, config parsing, live session setup, desktop launch
- **C/C++** — Only used where Rust cannot go (e.g., direct kernel ioctls, legacy library FFI that lacks Rust bindings)

| Component | Language | Responsibility |
|-----------|----------|---------------|
| `blunux-wizard` | Rust | Hardware detection, config.toml loading, locale/keyboard setup, exec desktop |
| `blunux-toml2cal` | Rust | config.toml → Calamares YAML translation |
| Low-level fallback | C | Kernel-level hardware probing where no Rust crate exists |

#### Wizard Flow

```bash
# /usr/bin/startblunux (simplified)
#!/bin/bash

# Rust binary handles everything directly — no Julia, no FFI layers.
blunux-wizard

# blunux-wizard internally:
#   1. Detects GPU, audio, UEFI, RAM via /sys and /proc
#   2. Auto-selects drivers (NVIDIA→proprietary, AMD/Intel→mesa)
#   3. Loads config.toml
#   4. Applies locale and keyboard to live session
#   5. Execs startplasma-wayland (or startplasma-x11 as fallback)
#
# Theme: single curated blunux2 theme, no user selection.
# Drivers: auto-detected, no user selection.
```

**Key design principle:** All wizard selections are written to `config.toml`. When the user clicks "Install", the Rust-based translator (`blunux-toml2cal`) reads `config.toml` and generates the full set of Calamares YAML configuration files automatically. The user never touches Calamares config directly — `config.toml` is the single source of truth.

### 4.6 config.toml — User-Facing Configuration

The user interacts exclusively with `config.toml` for both the live session wizard and the disk installer. This file is designed to be human-readable and editable, using TOML syntax with Korean comments.

```toml
# Example: config.toml (abbreviated)
[blunux]
version = "2.0"

[locale]
language = ["ko_KR"]
timezone = "Europe/Stockholm"
keyboard = ["kr", "us"]

[install]
bootloader = "nmbl"
hostname = "nux"
username = "blu"
encryption = false

[disk]
swap = "small"          # none / small / suspend / file

[packages.desktop]
kde = true

[packages.browser]
firefox = true
```

**Design decisions:**
- **Theme** — Single curated blunux2 theme ships by default. No theme selection in the wizard or config.toml.
- **Drivers** — Auto-detected at boot by Rust hw-detect (NVIDIA → proprietary, AMD/Intel → mesa). No user selection needed.
- **Filesystem** — ext4 is the only supported layout (hardcoded in the translator). Chosen for simplicity and stability.

When the user is satisfied with their configuration (either via the GUI wizard or by editing `config.toml` directly), the installation proceeds as follows:

```
┌─────────────┐     ┌──────────────────┐     ┌───────────────────┐
│ config.toml │────▶│ blunux-toml2cal   │────▶│ Calamares YAML    │
│ (user edits)│     │ (Rust translator) │     │ settings.conf     │
│             │     │                   │     │ partition.conf    │
│             │     │                   │     │ unpackfs.conf     │
│             │     │                   │     │ locale.conf       │
│             │     │                   │     │ users.conf        │
│             │     │                   │     │ bootloader.conf   │
└─────────────┘     └──────────────────┘     └───────────────────┘
                                                      │
                                                      ▼
                                              ┌───────────────────┐
                                              │ Calamares runs     │
                                              │ standard pipeline  │
                                              └───────────────────┘
```

#### Translation Rules (config.toml → Calamares)

The Rust translator (`blunux-toml2cal`) maps TOML sections to Calamares module configs:

| config.toml section | Calamares module | Generated file |
|---------------------|-----------------|----------------|
| `[locale]` | locale, keyboard | `locale.conf`, `keyboard.conf` |
| `[install]` bootloader | bootloader | `bootloader.conf` |
| `[install]` hostname/username | users | `users.conf` |
| `[install]` encryption | partition | `partition.conf` (LUKS settings) |
| `[disk]` swap | partition | `partition.conf` (swap choice) |
| `[kernel]` | shellprocess | kernel install commands |
| `[packages.*]` | shellprocess | post-install package list |
| `[input_method]` | shellprocess | input method setup commands |

This "click-to-install" approach means a user can configure everything in a single TOML file, click install, and the Rust translator handles the rest — no manual Calamares configuration needed.

---

## 5. Subsystem 3: Disk Installation (Calamares)

### 5.1 Calamares Overview

**Calamares** is a universal Linux installer framework. It's modular — you configure which modules run and in what order.

In blunux2, users never interact with Calamares configuration directly. Instead, all install preferences are stored in a single **`config.toml`** file. A Rust-based translator (`blunux-toml2cal`) converts this TOML into the full set of Calamares YAML configs at install time. This "click-to-install" approach means the user configures everything in one readable file, clicks install, and the system handles the rest.

### 5.2 Calamares Module Pipeline

```yaml
# /etc/calamares/settings.conf

modules-search: [ local, /usr/lib/calamares/modules ]

sequence:
  - show:                    # ── UI Screens ──
    - welcome               # Language + requirements check
    - locale                 # Timezone + locale
    - keyboard               # Keyboard layout
    - partition              # Disk partitioning
    - users                  # Username + password
    - summary                # Review before install

  - exec:                    # ── Installation Steps ──
    - partition              # Create partitions (ext4 default)
    - mount                  # Mount target partitions
    - unpackfs               # Extract squashfs → target disk
    - machineid              # Generate /etc/machine-id
    - fstab                  # Generate /etc/fstab
    - locale                 # Write locale config
    - keyboard               # Write keyboard config
    - localecfg              # Write /etc/locale.gen
    - luksbootkeyfile        # LUKS encryption key
    - users                  # Create user accounts
    - displaymanager         # Configure SDDM
    - networkcfg             # Copy network config
    - hwclock                # Hardware clock
    - services-systemd       # Enable systemd services
    - shellprocess           # Run custom shell scripts
    - grubcfg                # Generate GRUB config
    - bootloader             # Install GRUB to disk
    - umount                 # Unmount everything

  - show:
    - finished               # "Installation complete" screen
```

### 5.3 The Critical Step: unpackfs (squashfs → disk)

This is the core of installation — extracting the live filesystem to the target disk:

```yaml
# /etc/calamares/modules/unpackfs.conf

unpack:
  - source: /run/miso/bootmnt/arch/x86_64/airootfs.sfs
    sourcefs: squashfs
    destination: ""  # root of target mount
```

**What happens internally:**
1. The squashfs image from the live media is mounted
2. `rsync` or `unsquashfs` extracts all files to the target partition
3. Live-session-specific files are excluded (e.g., archiso configs)
4. User's customizations from the wizard are preserved

### 5.4 Partition Configuration

```yaml
# /etc/calamares/modules/partition.conf

efiSystemPartition: /boot/efi
efiSystemPartitionSize: 512M
efiSystemPartitionName: EFI

defaultFileSystemType: ext4

swapChoices:
  - none
  - small      # RAM size for hibernation
  - suspend    # RAM size
  - file       # Swap file instead of partition
```

### 5.5 config.toml → Calamares Translation (Pre-Install)

Before Calamares runs, the Rust translator reads the user's `config.toml` and generates all required Calamares YAML configs:

```bash
# Called by calamares-blunux wrapper script before launching Calamares
blunux-toml2cal \
    --input /usr/share/blunux/config.toml \
    --output-dir /etc/calamares/modules/ \
    --settings /etc/calamares/settings.conf
```

The translator is a statically-linked Rust binary (`blunux-toml2cal`) that:
1. Parses `config.toml` using the `toml` crate
2. Maps each TOML section to the corresponding Calamares module config
3. Writes well-formed YAML files (using the `serde_yaml` crate)
4. Generates `settings.conf` with the correct module pipeline sequence

This means `config.toml` is the only file the user (or the wizard UI) needs to modify. Calamares receives fully-formed YAML and runs its standard pipeline.

### 5.6 Post-Install Scripts (shellprocess)

```yaml
# /etc/calamares/modules/shellprocess.conf
# (auto-generated by blunux-toml2cal from config.toml)

script:
  # Remove live-session packages
  - command: "chroot $ROOT pacman -Rns --noconfirm mkinitcpio-archiso"

  # Regenerate initramfs for installed system
  - command: "chroot $ROOT mkinitcpio -P"

  # Enable services
  - command: "chroot $ROOT systemctl enable sddm NetworkManager bluetooth"

  # Copy live session theme to installed system
  - command: "cp /home/liveuser/.config/plasma* $ROOT/etc/skel/.config/"

  # Install packages selected in config.toml [packages.*] sections
  - command: "chroot $ROOT blunux-toml2cal --apply-packages /usr/share/blunux/config.toml"

  # Configure input method from config.toml [input_method]
  - command: "chroot $ROOT blunux-toml2cal --apply-input-method /usr/share/blunux/config.toml"

  # Clean up
  - command: "chroot $ROOT pacman -Scc --noconfirm"

  # Set default kernel parameters
  - command: >
      sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT=.*/GRUB_CMDLINE_LINUX_DEFAULT="quiet splash"/'
      $ROOT/etc/default/grub
```

---

## 6. ISO Image Structure (Final Output)

```
blunux2-2026.02.13-x86_64.iso
├── arch/
│   ├── boot/
│   │   └── x86_64/
│   │       ├── vmlinuz-linux          # Linux kernel
│   │       └── initramfs-linux.img    # initramfs with archiso hooks
│   └── x86_64/
│       └── airootfs.sfs              # SquashFS compressed root (~2.5GB)
├── EFI/
│   └── BOOT/
│       ├── BOOTx64.EFI              # GRUB EFI binary
│       └── grub.cfg                  # GRUB config
├── syslinux/
│   ├── syslinux.cfg                 # BIOS boot config
│   ├── ldlinux.sys                  # Syslinux bootloader
│   └── splash.png                   # Boot splash screen
├── boot/
│   └── grub/
│       └── grub.cfg                 # Full GRUB config
└── [El Torito boot catalog]          # For CD/DVD booting
```

---

## 7. Key Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Base | Arch Linux (or Manjaro) | Rolling release, AUR access, latest packages |
| ISO tool | archiso + mkarchiso | Official, well-maintained, proven |
| Compression | SquashFS + zstd | Best compression ratio for live media |
| Root FS overlay | overlayfs (tmpfs upper) | Standard Linux overlay, volatile by design |
| Installer | Calamares (via config.toml) | Universal framework, driven by TOML→YAML translation |
| Install config | config.toml → Calamares YAML | User edits TOML; Rust translator generates Calamares configs |
| Default FS | ext4 | Simple, stable, battle-tested, universal support |
| Desktop | KDE Plasma 6 | Highly customizable, Wayland-ready |
| Display manager | SDDM | Native KDE integration |
| Setup wizard | Rust (`blunux-wizard`) | Hardware detection, config loading, live session setup, desktop launch |
| Config translator | Rust (`blunux-toml2cal`) | config.toml → Calamares YAML generation |
| Low-level fallback | C/C++ | Kernel ioctls, legacy library FFI without Rust bindings |
| UI toolkit | GTK4/Libadwaita (via gtk4-rs) | Modern UI, Rust-native bindings |
| ISO build orchestrator | Julia (`build.jl`) | Dev-side only — reads config.toml, generates archiso profile, calls mkarchiso |
| No Python | — | Zero Python dependency (build-side or ISO-side) |
| No Julia in ISO | — | Julia used only for build orchestration on developer machine |
| Boot (UEFI) | GRUB | Widest hardware compatibility |
| Boot (BIOS) | syslinux | Lightweight, reliable for legacy |
| CI/CD | GitHub Actions | Free for open-source, matrix builds |
| Package format | Arch packages (PKGBUILD) | AUR compatible, simple packaging |
| Custom repo | Self-hosted (repo.blunux2.dev) | Host blunux2-specific packages |

---

## 8. Custom Package Repository

blunux2 will need its own repository for distribution-specific packages:

```
blunux2-repo/
├── blunux2-settings/       # Default configs, branding
│   └── PKGBUILD
├── blunux2-livecd/         # Live session wizard (Rust binary)
│   └── PKGBUILD            #   cargo build --release → blunux-wizard
├── blunux2-toml2cal/       # config.toml → Calamares YAML translator (Rust)
│   └── PKGBUILD            #   cargo build --release → blunux-toml2cal
├── blunux2-themes/         # KDE themes, icons, wallpapers
│   └── PKGBUILD
├── blunux2-calamares/      # Calamares template configs (generated at install time)
│   └── PKGBUILD
├── blunux2-store/          # App store frontend (pamac/bigstore-like)
│   └── PKGBUILD
└── blunux2-welcome/        # Post-install welcome app
    └── PKGBUILD
```

Add to `pacman.conf`:
```ini
[blunux2]
SigLevel = Optional TrustAll
Server = https://repo.blunux2.dev/$arch
```

---

## 9. Development Workflow

### 9.1 Local Development Cycle

```bash
# 1. Edit profile (packages, airootfs configs, etc.)
vim blunux2-profile/packages.x86_64
vim blunux2-profile/airootfs/etc/skel/.config/...

# 2. Build ISO
sudo mkarchiso -v -w /tmp/work -o /tmp/out blunux2-profile/

# 3. Test in QEMU
run_archiso -u -i /tmp/out/blunux2-*.iso   # UEFI mode
run_archiso -i /tmp/out/blunux2-*.iso       # BIOS mode

# 4. Test on real hardware
dd bs=4M if=/tmp/out/blunux2-*.iso of=/dev/sdX status=progress
```

### 9.2 Release Process

1. Tag a version in git
2. GitHub Actions builds ISO matrix (stable/testing × minimal/full)
3. ISOs uploaded to GitHub Releases + mirror CDN
4. Generate `.torrent` files for community distribution
5. Update website ISO download links

---

## 10. Future Considerations

- **Persistent live USB** — Allow saving changes across reboots using a secondary partition with ext4
- **Netboot (PXE)** — archiso already supports PXE boot via the pxe hooks; enable for lab/classroom deployments
- **ARM64 port** — archiso supports ARM profiles for Raspberry Pi / ARM laptops
- **Immutable variant** — Consider an immutable/OSTree-based variant for enterprise use
- **Auto-update ISO** — Script to rebuild ISOs nightly from latest packages
- **Snapshot integration** — Timeshift integration for rollback
- **Korean/Swedish locale packs** — Pre-configured CJK input methods and Swedish keyboard layouts

---

## 11. References

- [Arch Wiki — archiso](https://wiki.archlinux.org/title/Archiso)
- [Arch Wiki — mkinitcpio](https://wiki.archlinux.org/title/Mkinitcpio)
- [Calamares Documentation](https://github.com/calamares/calamares/wiki)
- [BigLinux LiveCD (reference implementation)](https://github.com/biglinux/biglinux-livecd)
- [BigLinux Calamares Config](https://github.com/biglinux/calamares-biglinux)
- [BigLinux ISO Build Action](https://github.com/biglinux/biglinux-iso-action)
- [ALCI — Arch Linux Calamares Installer](https://alci.online/)
- [ArchISO Boot Process (DeepWiki)](https://deepwiki.com/archlinux/archiso/4.3-boot-process-flow)
