//! C-ABI FFI exports for Julia `ccall`.
//!
//! Julia calls these functions via:
//!   ccall((:blunux_detect_gpu, "libblunux"), Cint, ())

use crate::hwdetect;
use crate::config;
use libc::c_char;
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::Mutex;

// Global config handle — Julia loads it once, then reads/writes through FFI.
static CONFIG: Mutex<Option<blunux_config::BlunuxConfig>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Hardware detection
// ---------------------------------------------------------------------------

/// Detect GPU vendor. Returns: 0=NVIDIA, 1=AMD, 2=Intel, 3=Unknown.
#[no_mangle]
pub extern "C" fn blunux_detect_gpu() -> i32 {
    hwdetect::detect_gpu() as i32
}

/// Detect audio backend. Returns: 0=Pipewire, 1=PulseAudio, 2=None.
#[no_mangle]
pub extern "C" fn blunux_detect_audio() -> i32 {
    hwdetect::detect_audio() as i32
}

/// Check UEFI mode. Returns: 1=UEFI, 0=BIOS.
#[no_mangle]
pub extern "C" fn blunux_is_uefi() -> i32 {
    hwdetect::is_uefi() as i32
}

/// Get total system RAM in MB.
#[no_mangle]
pub extern "C" fn blunux_total_ram_mb() -> u64 {
    hwdetect::total_ram_mb()
}

/// Get recommended GPU driver packages as a newline-separated C string.
/// Caller must free with `blunux_free_string`.
#[no_mangle]
pub extern "C" fn blunux_gpu_driver_packages() -> *mut c_char {
    let vendor = hwdetect::detect_gpu();
    let pkgs = hwdetect::gpu_driver_packages(vendor);
    let joined = pkgs.join("\n");
    match CString::new(joined) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// ---------------------------------------------------------------------------
// Config management
// ---------------------------------------------------------------------------

/// Load config.toml into the global handle. Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn blunux_config_load(path: *const c_char) -> i32 {
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    match config::load_config(path_str) {
        Some(cfg) => {
            if let Ok(mut guard) = CONFIG.lock() {
                *guard = Some(cfg);
                0
            } else {
                -1
            }
        }
        None => -1,
    }
}

/// Save the global config back to a TOML file. Returns 0 on success.
#[no_mangle]
pub extern "C" fn blunux_config_save(path: *const c_char) -> i32 {
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if let Ok(guard) = CONFIG.lock() {
        if let Some(ref cfg) = *guard {
            if config::save_config(cfg, path_str) {
                return 0;
            }
        }
    }
    -1
}

/// Set locale language in the loaded config.
#[no_mangle]
pub extern "C" fn blunux_config_set_language(lang: *const c_char) -> i32 {
    let lang_str = match unsafe { CStr::from_ptr(lang) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if let Ok(mut guard) = CONFIG.lock() {
        if let Some(ref mut cfg) = *guard {
            config::set_locale_language(cfg, lang_str);
            return 0;
        }
    }
    -1
}

/// Set timezone in the loaded config.
#[no_mangle]
pub extern "C" fn blunux_config_set_timezone(tz: *const c_char) -> i32 {
    let tz_str = match unsafe { CStr::from_ptr(tz) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if let Ok(mut guard) = CONFIG.lock() {
        if let Some(ref mut cfg) = *guard {
            config::set_locale_timezone(cfg, tz_str);
            return 0;
        }
    }
    -1
}

/// Set hostname in the loaded config.
#[no_mangle]
pub extern "C" fn blunux_config_set_hostname(hostname: *const c_char) -> i32 {
    let hostname_str = match unsafe { CStr::from_ptr(hostname) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if let Ok(mut guard) = CONFIG.lock() {
        if let Some(ref mut cfg) = *guard {
            config::set_hostname(cfg, hostname_str);
            return 0;
        }
    }
    -1
}

/// Set username in the loaded config.
#[no_mangle]
pub extern "C" fn blunux_config_set_username(username: *const c_char) -> i32 {
    let username_str = match unsafe { CStr::from_ptr(username) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if let Ok(mut guard) = CONFIG.lock() {
        if let Some(ref mut cfg) = *guard {
            config::set_username(cfg, username_str);
            return 0;
        }
    }
    -1
}

/// Set swap preference in the loaded config.
#[no_mangle]
pub extern "C" fn blunux_config_set_swap(swap: *const c_char) -> i32 {
    let swap_str = match unsafe { CStr::from_ptr(swap) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };

    if let Ok(mut guard) = CONFIG.lock() {
        if let Some(ref mut cfg) = *guard {
            config::set_swap(cfg, swap_str);
            return 0;
        }
    }
    -1
}

/// Get a config value as a C string. Caller must free with `blunux_free_string`.
/// Keys: "hostname", "username", "bootloader", "swap", "language", "timezone"
#[no_mangle]
pub extern "C" fn blunux_config_get(key: *const c_char) -> *mut c_char {
    let key_str = match unsafe { CStr::from_ptr(key) }.to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    if let Ok(guard) = CONFIG.lock() {
        if let Some(ref cfg) = *guard {
            let val = match key_str {
                "hostname" => cfg.install.hostname.clone(),
                "username" => cfg.install.username.clone(),
                "bootloader" => cfg.install.bootloader.clone(),
                "swap" => cfg.disk.swap.clone(),
                "language" => cfg.locale.language.first().cloned().unwrap_or_default(),
                "timezone" => cfg.locale.timezone.clone(),
                "kernel" => cfg.kernel.kernel_type.clone(),
                "input_method_engine" => cfg.input_method.engine.clone(),
                _ => return ptr::null_mut(),
            };

            return match CString::new(val) {
                Ok(cs) => cs.into_raw(),
                Err(_) => ptr::null_mut(),
            };
        }
    }
    ptr::null_mut()
}

// ---------------------------------------------------------------------------
// Memory management
// ---------------------------------------------------------------------------

/// Free a C string allocated by this library.
#[no_mangle]
pub extern "C" fn blunux_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = CString::from_raw(ptr);
        }
    }
}
