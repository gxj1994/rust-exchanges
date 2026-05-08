//! Timestamp Migration Example
//!
//! This example demonstrates how to migrate from the old u64 timestamp API
//! to the new standardized i64 timestamp API in CCXT Rust.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example timestamp_migration_example
//! ```

#![allow(clippy::disallowed_methods)]

use ccxt_core::types::common::ohlcv_request::OhlcvRequest;
use ccxt_core::{
    prelude::*,
    time::{TimestampConversion, TimestampUtils},
};
use ccxt_exchanges::binance::Binance;
use rust_decimal_macros::dec;
use std::env;

// Include common logging macros
#[path = "common/mod.rs"]
mod common;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    log_title!("Timestamp Migration Example");
    // Initialize exchange
    let config = ExchangeConfig {
        api_key: env::var("BINANCE_API_KEY")
            .ok()
            .map(ccxt_core::SecretString::new),
        secret: env::var("BINANCE_API_SECRET")
            .ok()
            .map(ccxt_core::SecretString::new),
        sandbox: false,
        ..Default::default()
    };
    let exchange = Binance::new(config)?;

    // Example 1: Basic timestamp creation and usage
    example_timestamp_creation().await?;

    // Example 2: Data structure creation with i64 timestamps
    example_data_structures().await?;

    // Example 3: OHLCV fetching migration
    example_ohlcv_migration(&exchange).await?;

    // Example 4: Trade fetching migration
    example_trades_migration(&exchange).await?;

    // Example 5: Conversion utilities
    example_conversion_utilities().await?;

    // Example 6: Error handling
    example_error_handling().await?;

    log_complete!();
    Ok(())
}

/// Example 1: Basic timestamp creation and usage
async fn example_timestamp_creation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    log_section!("Example 1: Timestamp Creation");

    // OLD WAY (deprecated - don't do this)
    #[allow(clippy::cast_possible_truncation)]
    let _old_timestamp = chrono::Utc::now().timestamp_millis() as u64;
    log_warning!("Old way: casting to u64 (deprecated)");

    // NEW WAY (recommended)
    let new_timestamp: i64 = chrono::Utc::now().timestamp_millis();
    log_success!("New way: using i64 directly");
    log_field!("Current timestamp", format!("{} ms", new_timestamp));

    // Using timestamp utilities
    let util_timestamp = TimestampUtils::now_ms();
    log_success!("Using utilities: {} ms", util_timestamp);

    // Formatting timestamps
    let formatted = TimestampUtils::format_iso8601(new_timestamp);
    log_field!("Formatted", format!("{:?}", formatted));

    Ok(())
}

/// Example 2: Data structure creation with i64 timestamps
async fn example_data_structures() -> std::result::Result<(), Box<dyn std::error::Error>> {
    log_section!("Example 2: Data Structures");

    let timestamp: i64 = chrono::Utc::now().timestamp_millis();

    // Create ticker with i64 timestamp
    let ticker = Ticker::new(Symbol::new_unchecked("BTC/USDT"), timestamp);
    log_success!("Ticker created with i64 timestamp: {}", ticker.timestamp);

    // Create order book with i64 timestamp
    let orderbook = OrderBook::new(Symbol::new_unchecked("BTC/USDT"), timestamp);
    log_success!(
        "OrderBook created with i64 timestamp: {}",
        orderbook.timestamp
    );

    // Create trade with i64 timestamp
    let trade = Trade::new(
        Symbol::new_unchecked("BTC/USDT"),
        OrderSide::Buy,
        dec!(50000.0).into(),
        dec!(0.1).into(),
        timestamp,
    );
    log_success!("Trade created with i64 timestamp: {}", trade.timestamp);

    // Create OHLCV with i64 timestamp
    let ohlcv = Ohlcv::new(
        timestamp,
        dec!(49000.0).into(),
        dec!(51000.0).into(),
        dec!(48500.0).into(),
        dec!(50000.0).into(),
        dec!(1234.5).into(),
    );
    log_success!("OHLCV created with i64 timestamp: {}", ohlcv.timestamp);

    Ok(())
}

/// Example 3: OHLCV fetching migration
async fn example_ohlcv_migration(
    exchange: &Binance,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    log_section!("Example 3: OHLCV Fetching Migration");

    // OLD WAY (deprecated - still works but shows warnings)
    log_warning!("Old way (deprecated):");
    println!("  \x1b[90m// This would show deprecation warnings:\x1b[0m");
    println!("  \x1b[90m// exchange.fetch_ohlcv_u64(...)\x1b[0m");

    // NEW WAY 1: Basic method using fetch_ohlcv
    log_success!("New way 1: Basic method with fetch_ohlcv");
    let request = OhlcvRequest::builder()
        .symbol("BTC/USDT")
        .timeframe("1h")
        .limit(5)
        .build()
        .expect("Failed to build OHLCV request");
    match exchange.fetch_ohlcv(request).await {
        Ok(candles) => {
            log_success!("Fetched {} candles using basic method", candles.len());
            if let Some(first) = candles.first() {
                log_field!("First candle timestamp", first.timestamp);
            }
        }
        Err(e) => log_error!("Error: {}", e),
    }

    // NEW WAY 2: Parameter-based method (recommended for complex queries)
    log_success!("New way 2: Parameter-based method");
    let since: i64 = chrono::Utc::now().timestamp_millis() - (24 * 60 * 60 * 1000);

    let request = OhlcvRequest::builder()
        .symbol("BTC/USDT")
        .timeframe("1h")
        .since(since)
        .limit(10)
        .build()
        .expect("Failed to build OHLCV request");
    match exchange.fetch_ohlcv(request).await {
        Ok(candles) => {
            log_success!("Fetched {} candles using parameter method", candles.len());
            if let Some(first) = candles.first() {
                log_field!("First candle timestamp", first.timestamp);
                log_field!("Since parameter (i64)", since);
            }
        }
        Err(e) => log_error!("Error: {}", e),
    }

    Ok(())
}

/// Example 4: Trade fetching migration
async fn example_trades_migration(
    exchange: &Binance,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    log_section!("Example 4: Trade Fetching Migration");

    // OLD WAY (this signature no longer exists)
    log_warning!("Old way (no longer available):");
    println!(
        "  \x1b[90m// exchange.fetch_trades(\"BTC/USDT\", Some(5)) // This signature is gone\x1b[0m"
    );

    // NEW WAY 1: Basic trades (no parameters)
    log_success!("New way 1: Basic trades");
    match exchange.fetch_market_trades("BTC/USDT", None).await {
        Ok(trades) => {
            log_success!("Fetched {} trades using basic method", trades.len());
        }
        Err(e) => log_error!("Error: {}", e),
    }

    // NEW WAY 2: With limit only
    log_success!("New way 2: With limit");
    match exchange.fetch_market_trades("BTC/USDT", Some(5)).await {
        Ok(trades) => {
            log_success!("Fetched {} trades with limit", trades.len());
        }
        Err(e) => log_error!("Error: {}", e),
    }

    // NEW WAY 3: With timestamp filtering (i64)
    log_success!("New way 3: With timestamp filtering");
    let since: i64 = chrono::Utc::now().timestamp_millis() - (60 * 60 * 1000);
    log_info!("Using fetch_trades with limit parameter");
    match exchange.fetch_market_trades("BTC/USDT", Some(10)).await {
        Ok(trades) => {
            log_success!("Fetched {} trades with timestamp filter", trades.len());
            log_field!("Since parameter (i64)", since);
        }
        Err(e) => log_error!("Error: {}", e),
    }

    Ok(())
}

/// Example 5: Conversion utilities
#[allow(deprecated)]
async fn example_conversion_utilities() -> std::result::Result<(), Box<dyn std::error::Error>> {
    log_section!("Example 5: Conversion Utilities");

    // Convert u64 to i64
    let u64_timestamp = 1609459200000u64;
    match TimestampUtils::u64_to_i64(u64_timestamp) {
        Ok(i64_timestamp) => {
            log_success!(
                "u64 to i64 conversion: {} -> {}",
                u64_timestamp,
                i64_timestamp
            );
        }
        Err(e) => log_error!("Conversion error: {}", e),
    }

    // Convert Option<u64> to Option<i64>
    let u64_option = Some(1609459200000u64);
    match u64_option.to_i64() {
        Ok(i64_option) => {
            log_success!(
                "Option<u64> to Option<i64>: {:?} -> {:?}",
                u64_option,
                i64_option
            );
        }
        Err(e) => log_error!("Conversion error: {}", e),
    }

    // Validate timestamp
    let timestamp = 1609459200000i64;
    match TimestampUtils::validate_timestamp(timestamp) {
        Ok(validated) => {
            log_success!("Timestamp validation: {} is valid", validated);
        }
        Err(e) => log_error!("Validation error: {}", e),
    }

    // Parse timestamp from string
    match TimestampUtils::parse_timestamp("1609459200000") {
        Ok(parsed) => {
            log_success!("String parsing: '1609459200000' -> {}", parsed);
        }
        Err(e) => log_error!("Parse error: {}", e),
    }

    // Time conversions
    let seconds = 1609459200i64;
    let milliseconds = TimestampUtils::seconds_to_ms(seconds);
    let back_to_seconds = TimestampUtils::ms_to_seconds(milliseconds);
    log_success!(
        "Time conversions: {}s -> {}ms -> {}s",
        seconds,
        milliseconds,
        back_to_seconds
    );

    Ok(())
}

/// Example 6: Error handling
#[allow(deprecated)]
async fn example_error_handling() -> std::result::Result<(), Box<dyn std::error::Error>> {
    log_section!("Example 6: Error Handling");

    // Test timestamp overflow (u64 to i64)
    let large_u64 = u64::MAX;
    match TimestampUtils::u64_to_i64(large_u64) {
        Ok(converted) => {
            log_warning!("Unexpected success: {}", converted);
        }
        Err(e) => {
            log_success!("Correctly caught overflow: {}", e);
        }
    }

    // Test timestamp underflow (i64 to u64)
    let negative_i64 = -1000i64;
    match TimestampUtils::i64_to_u64(negative_i64) {
        Ok(converted) => {
            log_warning!("Unexpected success: {}", converted);
        }
        Err(e) => {
            log_success!("Correctly caught underflow: {}", e);
        }
    }

    // Test invalid timestamp range
    let invalid_timestamp = -1i64;
    match TimestampUtils::validate_timestamp(invalid_timestamp) {
        Ok(validated) => {
            log_warning!("Unexpected success: {}", validated);
        }
        Err(e) => {
            log_success!("Correctly caught invalid range: {}", e);
        }
    }

    // Test invalid string parsing
    match TimestampUtils::parse_timestamp("not_a_number") {
        Ok(parsed) => {
            log_warning!("Unexpected success: {}", parsed);
        }
        Err(e) => {
            log_success!("Correctly caught parse error: {}", e);
        }
    }

    Ok(())
}
