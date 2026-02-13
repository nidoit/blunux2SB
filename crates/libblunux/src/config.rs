use blunux_config::BlunuxConfig;
use std::path::Path;

/// Load BlunuxConfig from a file. Returns None on error.
pub fn load_config(path: &str) -> Option<BlunuxConfig> {
    BlunuxConfig::load(Path::new(path)).ok()
}

/// Save BlunuxConfig to a file. Returns true on success.
pub fn save_config(config: &BlunuxConfig, path: &str) -> bool {
    config.save(Path::new(path)).is_ok()
}

/// Update a specific field in the config. Used by the wizard to apply user
/// selections one at a time.
pub fn set_locale_language(config: &mut BlunuxConfig, lang: &str) {
    config.locale.language = vec![lang.to_string()];
}

pub fn set_locale_timezone(config: &mut BlunuxConfig, tz: &str) {
    config.locale.timezone = tz.to_string();
}

pub fn set_locale_keyboard(config: &mut BlunuxConfig, layouts: Vec<String>) {
    config.locale.keyboard = layouts;
}

pub fn set_hostname(config: &mut BlunuxConfig, hostname: &str) {
    config.install.hostname = hostname.to_string();
}

pub fn set_username(config: &mut BlunuxConfig, username: &str) {
    config.install.username = username.to_string();
}

pub fn set_swap(config: &mut BlunuxConfig, swap: &str) {
    config.disk.swap = swap.to_string();
}
