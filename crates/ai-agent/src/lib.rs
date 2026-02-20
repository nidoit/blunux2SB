// lib.rs — re-exports all modules so integration tests in tests/ can import
// them as `ai_agent::module::Type`.
//
// The [[bin]] target (main.rs) declares its own `mod` tree independently;
// both compile from the same source files but as separate compilation units.

pub mod agent;
pub mod config;
pub mod daemon;
pub mod error;
pub mod ipc;
pub mod memory;
pub mod providers;
pub mod setup;
pub mod strings;
pub mod tools;
