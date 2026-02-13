#!/usr/bin/env julia
#
# blunux2 Setup Wizard — Julia Orchestrator
#
# Calls into libblunux.so (Rust) via ccall for:
#   - Hardware detection (GPU, audio, UEFI)
#   - Config loading/saving (config.toml)
#   - Driver auto-selection
#
# No Python anywhere in this pipeline.

const LIB = "libblunux"
const CONFIG_PATH = "/usr/share/blunux/config.toml"

# ---------------------------------------------------------------------------
# FFI wrappers around libblunux.so
# ---------------------------------------------------------------------------

module HwDetect
    using Main: LIB

    """Detect GPU vendor: 0=NVIDIA, 1=AMD, 2=Intel, 3=Unknown"""
    function detect_gpu()::Int32
        ccall((:blunux_detect_gpu, LIB), Cint, ())
    end

    """Detect audio backend: 0=Pipewire, 1=PulseAudio, 2=None"""
    function detect_audio()::Int32
        ccall((:blunux_detect_audio, LIB), Cint, ())
    end

    """Check if booted in UEFI mode"""
    function is_uefi()::Bool
        ccall((:blunux_is_uefi, LIB), Cint, ()) == 1
    end

    """Get total system RAM in MB"""
    function total_ram_mb()::UInt64
        ccall((:blunux_total_ram_mb, LIB), Culonglong, ())
    end

    """Get GPU driver packages as a vector of strings"""
    function gpu_driver_packages()::Vector{String}
        ptr = ccall((:blunux_gpu_driver_packages, LIB), Ptr{Cchar}, ())
        if ptr == C_NULL
            return String[]
        end
        result = unsafe_string(ptr)
        ccall((:blunux_free_string, LIB), Cvoid, (Ptr{Cchar},), ptr)
        return split(result, "\n")
    end

    const GPU_NAMES = Dict(0 => "NVIDIA", 1 => "AMD", 2 => "Intel", 3 => "Unknown")

    function gpu_name(vendor_id::Int32)::String
        get(GPU_NAMES, vendor_id, "Unknown")
    end
end

module Config
    using Main: LIB, CONFIG_PATH

    """Load config.toml into the Rust global handle"""
    function load(path::String=CONFIG_PATH)::Bool
        ccall((:blunux_config_load, LIB), Cint, (Cstring,), path) == 0
    end

    """Save config back to a TOML file"""
    function save(path::String=CONFIG_PATH)::Bool
        ccall((:blunux_config_save, LIB), Cint, (Cstring,), path) == 0
    end

    """Set a config value by calling the appropriate Rust FFI setter"""
    function set_language(lang::String)::Bool
        ccall((:blunux_config_set_language, LIB), Cint, (Cstring,), lang) == 0
    end

    function set_timezone(tz::String)::Bool
        ccall((:blunux_config_set_timezone, LIB), Cint, (Cstring,), tz) == 0
    end

    function set_hostname(name::String)::Bool
        ccall((:blunux_config_set_hostname, LIB), Cint, (Cstring,), name) == 0
    end

    function set_username(name::String)::Bool
        ccall((:blunux_config_set_username, LIB), Cint, (Cstring,), name) == 0
    end

    function set_swap(swap::String)::Bool
        ccall((:blunux_config_set_swap, LIB), Cint, (Cstring,), swap) == 0
    end

    """Get a config value as a string"""
    function get(key::String)::String
        ptr = ccall((:blunux_config_get, LIB), Ptr{Cchar}, (Cstring,), key)
        if ptr == C_NULL
            return ""
        end
        result = unsafe_string(ptr)
        ccall((:blunux_free_string, LIB), Cvoid, (Ptr{Cchar},), ptr)
        return result
    end
end

# ---------------------------------------------------------------------------
# Wizard steps
# ---------------------------------------------------------------------------

function step_hardware_detect()
    println("── Hardware Detection ──")

    gpu = HwDetect.detect_gpu()
    gpu_name = HwDetect.gpu_name(gpu)
    println("  GPU: $gpu_name")

    drivers = HwDetect.gpu_driver_packages()
    println("  Auto-selected drivers: $(join(drivers, ", "))")

    audio = HwDetect.detect_audio()
    audio_name = audio == 0 ? "Pipewire" : audio == 1 ? "PulseAudio" : "None"
    println("  Audio: $audio_name")

    uefi = HwDetect.is_uefi()
    println("  Boot mode: $(uefi ? "UEFI" : "BIOS")")

    ram = HwDetect.total_ram_mb()
    println("  RAM: $(ram) MB")

    return (gpu=gpu, uefi=uefi, ram=ram)
end

function step_load_config()
    println("\n── Loading Configuration ──")

    if !Config.load()
        error("Failed to load $CONFIG_PATH")
    end

    println("  Language: $(Config.get("language"))")
    println("  Timezone: $(Config.get("timezone"))")
    println("  Hostname: $(Config.get("hostname"))")
    println("  Username: $(Config.get("username"))")
    println("  Bootloader: $(Config.get("bootloader"))")
    println("  Swap: $(Config.get("swap"))")
    println("  Kernel: $(Config.get("kernel"))")
end

function step_apply_locale()
    println("\n── Applying Locale ──")

    lang = Config.get("language")
    tz = Config.get("timezone")

    # Apply locale to live session
    run(`localectl set-locale LANG=$(lang).UTF-8`, wait=false)
    run(`timedatectl set-timezone $tz`, wait=false)

    println("  Applied: LANG=$(lang).UTF-8, TZ=$tz")
end

function step_apply_keyboard()
    println("\n── Applying Keyboard Layout ──")
    # Keyboard layouts are configured through config.toml [locale].keyboard
    # and applied by Calamares during install. For the live session, we
    # use localectl.
    println("  Keyboard configured via config.toml")
end

function step_save_config()
    println("\n── Saving Configuration ──")

    if !Config.save()
        error("Failed to save config")
    end
    println("  Config saved to $CONFIG_PATH")
end

# ---------------------------------------------------------------------------
# Main orchestrator
# ---------------------------------------------------------------------------

function main()
    println("╔══════════════════════════════════════╗")
    println("║     blunux2 Setup Wizard v2.0        ║")
    println("║     Julia + Rust + C  (no Python)    ║")
    println("╚══════════════════════════════════════╝")
    println()

    # 1. Detect hardware
    hw = step_hardware_detect()

    # 2. Load config.toml (may have been pre-configured via web builder)
    step_load_config()

    # 3. Apply live session settings
    step_apply_locale()
    step_apply_keyboard()

    # 4. Save any modifications back to config.toml
    step_save_config()

    # 5. Launch desktop session
    println("\n── Launching Desktop ──")
    bootloader = Config.get("bootloader")
    println("  Bootloader: $bootloader")
    println("  Starting Plasma Wayland session...")

    # Replace this process with the desktop session
    # exec() replaces the current process — Julia exits, Plasma takes over.
    ccall(:execvp, Cint, (Cstring, Ptr{Cstring}),
        "startplasma-wayland", C_NULL)

    # If execvp returns, something went wrong — fall back to X11
    println("  Wayland failed, falling back to X11...")
    ccall(:execvp, Cint, (Cstring, Ptr{Cstring}),
        "startplasma-x11", C_NULL)
end

main()
