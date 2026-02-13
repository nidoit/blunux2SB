use std::fs;
use std::path::Path;

/// Detected GPU vendor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Nvidia = 0,
    Amd = 1,
    Intel = 2,
    Unknown = 3,
}

/// Detected audio backend.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioBackend {
    Pipewire = 0,
    PulseAudio = 1,
    None = 2,
}

/// Detect primary GPU vendor by scanning /sys/class/drm/card*/device/vendor.
pub fn detect_gpu() -> GpuVendor {
    let drm_path = Path::new("/sys/class/drm");
    if !drm_path.exists() {
        return GpuVendor::Unknown;
    }

    let entries = match fs::read_dir(drm_path) {
        Ok(e) => e,
        Err(_) => return GpuVendor::Unknown,
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with("card") || name_str.contains('-') {
            continue;
        }

        let vendor_path = entry.path().join("device/vendor");
        if let Ok(vendor_id) = fs::read_to_string(&vendor_path) {
            let vendor_id = vendor_id.trim();
            match vendor_id {
                "0x10de" => return GpuVendor::Nvidia,
                "0x1002" => return GpuVendor::Amd,
                "0x8086" => return GpuVendor::Intel,
                _ => {}
            }
        }
    }

    GpuVendor::Unknown
}

/// Return the list of driver packages to install for the detected GPU.
/// Drivers are auto-selected: NVIDIA → proprietary, AMD/Intel → mesa.
pub fn gpu_driver_packages(vendor: GpuVendor) -> Vec<&'static str> {
    match vendor {
        GpuVendor::Nvidia => vec![
            "nvidia-dkms",
            "nvidia-utils",
            "lib32-nvidia-utils",
            "nvidia-settings",
        ],
        GpuVendor::Amd => vec![
            "mesa",
            "vulkan-radeon",
            "lib32-mesa",
            "lib32-vulkan-radeon",
            "xf86-video-amdgpu",
        ],
        GpuVendor::Intel => vec![
            "mesa",
            "vulkan-intel",
            "lib32-mesa",
            "lib32-vulkan-intel",
            "intel-media-driver",
        ],
        GpuVendor::Unknown => vec!["mesa"],
    }
}

/// Check if audio hardware is present via /proc/asound.
pub fn detect_audio() -> AudioBackend {
    if Path::new("/proc/asound/cards").exists() {
        // Default to pipewire on modern systems
        AudioBackend::Pipewire
    } else {
        AudioBackend::None
    }
}

/// Check if the system is booted in UEFI mode.
pub fn is_uefi() -> bool {
    Path::new("/sys/firmware/efi").exists()
}

/// Get total system RAM in megabytes.
pub fn total_ram_mb() -> u64 {
    let meminfo = match fs::read_to_string("/proc/meminfo") {
        Ok(s) => s,
        Err(_) => return 0,
    };

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            // Format: "MemTotal:       16384000 kB"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(kb_str) = parts.get(1) {
                if let Ok(kb) = kb_str.parse::<u64>() {
                    return kb / 1024;
                }
            }
        }
    }
    0
}
