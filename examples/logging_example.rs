//! Logging System Usage Example
//!
//! Demonstrates how to configure and use the rust-exchanges structured logging system.
//!
//! # Usage
//!
//! ```bash
//! # Run with different log levels
//! RUST_LOG=debug cargo run --example logging_example
//! RUST_LOG=ccxt_core=trace cargo run --example logging_example
//! ```

#![allow(clippy::disallowed_methods)]

use ccxt_core::{
    ExchangeConfig,
    error::Result,
    logging::{LogConfig, LogFormat, LogLevel, init_logging},
};
use ccxt_exchanges::binance::Binance;

// Include common logging macros
#[path = "common/mod.rs"]
mod common;

#[tokio::main]
async fn main() -> Result<()> {
    log_title!("rust-exchanges Logging System");

    log_section!("Default Configuration");
    log_info!("Using Info level, Pretty format");
    init_logging(&LogConfig::default());
    log_success!("Logging system initialized");

    let mut config = ExchangeConfig::default();
    config.verbose = true;
    let exchange = Binance::new(config)?;

    log_step!("1", "Executing API call to generate log output...");
    match exchange
        .fetch_ticker("BTC/USDT", ccxt_core::types::TickerParams::default())
        .await
    {
        Ok(ticker) => {
            log_success!("Fetched ticker: {} @ {:?}", ticker.symbol, ticker.last);
        }
        Err(e) => {
            log_error!("Failed to fetch ticker: {}", e);
        }
    }

    log_section!("Development Configuration");
    log_info!("Debug level with span events");
    log_item!("Log level: Debug");
    log_item!("Format: Pretty");
    log_item!("Show time: Yes");
    log_item!("Show target: Yes");
    log_item!("Show span events: Yes");

    log_section!("Production Configuration");
    log_info!("Info level with JSON format");
    log_item!("Log level: Info");
    log_item!("Format: Json");
    log_item!("Show time: Yes");
    log_item!("Show thread IDs: Yes");
    log_item!("Show target: Yes");
    log_item!("Show span events: No");

    log_section!("Custom Configuration");
    let _custom_config = LogConfig {
        level: LogLevel::Warn,
        format: LogFormat::Compact,
        show_time: true,
        show_thread_ids: false,
        show_target: true,
        show_span_events: false,
    };
    log_info!("Custom log configuration");
    log_item!("Log level: Warn (warnings and errors only)");
    log_item!("Format: Compact");

    log_section!("Environment Variables");
    log_info!("Override configuration with RUST_LOG:");
    println!();
    println!("  \x1b[90m# Set global log level to debug\x1b[0m");
    println!("  \x1b[90mexport RUST_LOG=debug\x1b[0m");
    println!();
    println!("  \x1b[90m# Show debug logs for ccxt_core module only\x1b[0m");
    println!("  \x1b[90mexport RUST_LOG=ccxt_core=debug\x1b[0m");
    println!();
    println!("  \x1b[90m# Set different levels for different modules\x1b[0m");
    println!("  \x1b[90mexport RUST_LOG=warn,ccxt_core::http_client=debug\x1b[0m");
    println!();

    log_section!("Log Level Descriptions");
    log_info!("From most to least verbose:");
    println!();
    log_item!("TRACE - Most detailed debugging");
    println!("      └─ Function entry/exit, loop iterations");
    log_item!("DEBUG - Development debugging");
    println!("      └─ HTTP request/response details");
    log_item!("INFO  - Important business events");
    println!("      └─ API call success, order creation");
    log_item!("WARN  - Warning messages");
    println!("      └─ Request retry, recoverable errors");
    log_item!("ERROR - Error messages");
    println!("      └─ API call failure, connection errors");

    log_section!("Log Output Formats");
    log_subsection!("Pretty format (development)");
    println!("  ```");
    println!("  2024-01-20T10:30:45.123Z DEBUG ccxt_core::http_client: HTTP request");
    println!("    method: GET");
    println!("    url: https://api.binance.com/api/v3/ticker/24hr");
    println!("  ```");
    log_subsection!("Compact format (space-saving)");
    println!("  ```");
    println!(
        "  2024-01-20T10:30:45.123Z DEBUG ccxt_core::http_client: HTTP request method=GET url=https://..."
    );
    println!("  ```");
    log_subsection!("JSON format (production)");
    println!("  ```json");
    println!(
        "  {{\"timestamp\":\"2024-01-20T10:30:45.123Z\",\"level\":\"DEBUG\",\"target\":\"ccxt_core::http_client\",\"fields\":{{\"message\":\"HTTP request\",\"method\":\"GET\"}}}}"
    );
    println!("  ```");

    log_section!("Best Practices");
    log_success!("Initialize logging at application startup");
    log_success!("Use LogConfig::development() for development");
    log_success!("Use LogConfig::production() for production");
    log_success!("Control verbose logging with ExchangeConfig::verbose");
    log_success!("Adjust log levels with environment variables");
    log_success!("Use JSON format in production");
    log_warning!("Avoid low-level logging in high-frequency loops");

    log_section!("Complete Application Example");
    println!("```rust");
    println!("use ccxt_core::{{");
    println!("    error::Result,");
    println!("    logging::{{init_logging, LogConfig}},");
    println!("    ExchangeConfig,");
    println!("}};");
    println!("use ccxt_exchanges::binance::Binance;");
    println!();
    println!("#[tokio::main]");
    println!("async fn main() -> Result<()> {{");
    println!("    // Initialize logging system");
    println!("    #[cfg(debug_assertions)]");
    println!("    init_logging(LogConfig::development());");
    println!("    ");
    println!("    // Create exchange and execute operations");
    println!("    let exchange = Binance::new(config)?;");
    println!("    let ticker = exchange.fetch_ticker(\"BTC/USDT\").await?;");
    println!("    Ok(())");
    println!("}}");
    println!("```");

    log_complete!();
    log_info!("Try: RUST_LOG=debug cargo run --example logging_example");

    Ok(())
}
