//! Common utilities for examples
//!
//! Provides standardized logging macros and helper functions for all examples.

#![allow(clippy::disallowed_methods)]

/// Print a success message with a green checkmark
#[macro_export]
macro_rules! log_success {
    ($($arg:tt)*) => {
        println!("\x1b[32m✓\x1b[0m {}", format!($($arg)*))
    };
}

/// Print an error message with a red cross
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        println!("\x1b[31m✗\x1b[0m {}", format!($($arg)*))
    };
}

/// Print a warning message with a yellow warning sign
#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        println!("\x1b[33m⚠\x1b[0m {}", format!($($arg)*))
    };
}

/// Print an info message with a blue info sign
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        println!("\x1b[34mℹ\x1b[0m {}", format!($($arg)*))
    };
}

/// Print a section header with a decorative border
#[macro_export]
macro_rules! log_section {
    ($($arg:tt)*) => {
        println!("\n\x1b[1;36m▶\x1b[0m \x1b[1m{}\x1b[0m", format!($($arg)*));
        println!("\x1b[36m{:=<60}\x1b[0m", "")
    };
}

/// Print a subsection header
#[macro_export]
macro_rules! log_subsection {
    ($($arg:tt)*) => {
        println!("\n  \x1b[1;33m●\x1b[0m \x1b[1m{}\x1b[0m", format!($($arg)*))
    };
}

/// Print a step indicator
#[macro_export]
macro_rules! log_step {
    ($step:expr, $($arg:tt)*) => {
        println!("    \x1b[90m[{}]\x1b[0m {}", $step, format!($($arg)*))
    };
}

/// Print a result item with indentation
#[macro_export]
macro_rules! log_item {
    ($($arg:tt)*) => {
        println!("      • {}", format!($($arg)*))
    };
}

/// Print a code/field name
#[macro_export]
macro_rules! log_field {
    ($name:expr, $value:expr) => {
        println!("      \x1b[90m{}:\x1b[0m {}", $name, $value)
    };
}

/// Print the main title of an example
#[macro_export]
macro_rules! log_title {
    ($($arg:tt)*) => {
        println!("\n\x1b[1;35m╔══════════════════════════════════════════════════════════╗\x1b[0m");
        println!("\x1b[1;35m║\x1b[0m {:^56} \x1b[1;35m║\x1b[0m", format!($($arg)*));
        println!("\x1b[1;35m╚══════════════════════════════════════════════════════════╝\x1b[0m\n")
    };
}

/// Print a completion message
#[macro_export]
macro_rules! log_complete {
    () => {
        println!("\n\x1b[1;32m✓ All examples completed successfully\x1b[0m\n")
    };
}

/// Print a skipped message
#[macro_export]
macro_rules! log_skipped {
    ($($arg:tt)*) => {
        println!("\x1b[33m⏸\x1b[0m Skipped: {}", format!($($arg)*))
    };
}

/// Print a divider line
#[macro_export]
macro_rules! log_divider {
    () => {
        println!("\x1b[90m{:-<60}\x1b[0m", "")
    };
}

/// Helper function to print exchange info header
#[allow(dead_code)]
pub fn print_exchange_info(name: &str, id: &str, version: &str) {
    println!();
    log_section!("Exchange Information");
    log_field!("Name", name);
    log_field!("ID", id);
    log_field!("Version", version);
}

/// Helper function to print a warning about credentials
#[allow(dead_code)]
pub fn print_credential_warning(exchange_name: &str) {
    println!();
    log_warning!(
        "Authentication required: Set {}_API_KEY and {}_API_SECRET environment variables",
        exchange_name.to_uppercase(),
        exchange_name.to_uppercase()
    );
    log_info!("This example will skip authenticated operations");
}

/// Helper function to print environment setup instructions
#[allow(dead_code)]
pub fn print_env_setup(exchange_name: &str, extra_vars: &[&str]) {
    log_section!("Environment Setup");
    log_info!("Set the following environment variables:");
    println!();
    println!(
        "  \x1b[90mexport {}_API_KEY=\"your_api_key\"\x1b[0m",
        exchange_name.to_uppercase()
    );
    println!(
        "  \x1b[90mexport {}_API_SECRET=\"your_api_secret\"\x1b[0m",
        exchange_name.to_uppercase()
    );
    for var in extra_vars {
        println!(
            "  \x1b[90mexport {}=\"your_{}\"\x1b[0m",
            var,
            var.to_lowercase()
        );
    }
    println!();
}

/// Re-export all macros for convenience
pub mod prelude {
    #![allow(unused_imports)]
    pub use crate::{
        log_complete, log_divider, log_error, log_field, log_info, log_item, log_section,
        log_skipped, log_step, log_subsection, log_success, log_title, log_warning,
    };
}
