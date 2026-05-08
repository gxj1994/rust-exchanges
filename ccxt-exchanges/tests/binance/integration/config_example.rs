// Integration test example demonstrating TestConfig usage
//
// This test shows how to:
// - Load test configuration from environment
// - Use skip_if! and require_credentials! macros
// - Load test fixtures from JSON files
// - Write integration tests for both public and private APIs

// Allow clippy warnings for test code
#![allow(clippy::disallowed_methods)]

use crate::support::TestConfig;
use crate::{require_credentials, skip_if};
use serde_json::Value;
use std::fs;

// Fixtures路径常量 - 相对于测试执行时的当前目录
const FIXTURES_DIR: &str = "tests/binance/fixtures";

#[test]
fn test_load_config_from_env() {
    // Load configuration from environment variables
    // This will use .env file if present, otherwise use defaults
    let config = TestConfig::from_env().expect("Failed to load test config");

    // Verify configuration structure
    assert!(config.test_timeout_ms > 0);

    println!("✓ Config loaded successfully");
    println!("  - Timeout: {}ms", config.test_timeout_ms);
    println!("  - Skip private tests: {}", config.skip_private_tests);
}

#[test]
fn test_skip_if_macro() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    // Example: Skip test if private tests are disabled
    skip_if!(
        config,
        config.should_skip_private_tests(),
        "Private tests are disabled"
    );

    // This code only runs if private tests are enabled
    println!("✓ Private tests are enabled, test continues...");
}

#[test]
fn test_has_credentials() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    // Check if we have Binance credentials
    let has_binance = config.has_binance_credentials();
    println!("✓ Has Binance credentials: {}", has_binance);

    // Check if we have OKX credentials
    let has_okx = config.has_okx_credentials();
    println!("✓ Has OKX credentials: {}", has_okx);

    // Get active API key for Binance (respects testnet setting)
    if let Some((api_key, _secret)) = config.get_active_api_key("binance") {
        println!("✓ Active Binance API key: {}...", &api_key[..8]);
    }
}

#[test]
fn test_load_ticker_fixture() {
    // 直接使用相对路径加载fixture
    let fixture_path = format!("{}/tickers.json", FIXTURES_DIR);

    // Load and parse fixture
    let fixture_content = fs::read_to_string(&fixture_path).expect("Failed to read ticker fixture");

    let ticker: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse ticker JSON");

    // Verify fixture structure
    assert!(ticker["symbol"].as_str().is_some() || ticker["BTC/USDT"].is_object());

    println!("✓ Ticker fixture loaded successfully");
}

#[test]
fn test_load_orderbook_fixture() {
    let fixture_path = format!("{}/orderbooks.json", FIXTURES_DIR);
    let fixture_content =
        fs::read_to_string(&fixture_path).expect("Failed to read orderbook fixture");

    let orderbook: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse orderbook JSON");

    // Verify orderbook structure
    assert!(orderbook["bids"].as_array().is_some() || orderbook["BTC/USDT"]["bids"].is_array());
    assert!(orderbook["asks"].as_array().is_some() || orderbook["BTC/USDT"]["asks"].is_array());

    println!("✓ Orderbook fixture loaded successfully");
}

#[test]
fn test_load_trades_fixture() {
    let fixture_path = format!("{}/trades.json", FIXTURES_DIR);
    let fixture_content = fs::read_to_string(&fixture_path).expect("Failed to read trades fixture");

    let trades: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse trades JSON");

    let trades_array = trades.as_array().expect("Trades should be an array");
    assert!(!trades_array.is_empty());

    // Verify first trade structure
    let first_trade = &trades_array[0];
    assert!(first_trade["id"].as_i64().is_some() || first_trade["id"].as_str().is_some());
    assert!(first_trade["symbol"].as_str().is_some());

    println!("✓ Trades fixture loaded successfully");
    println!("  - Trade count: {}", trades_array.len());
}

#[test]
fn test_load_markets_fixture() {
    let fixture_path = format!("{}/markets.json", FIXTURES_DIR);
    let fixture_content =
        fs::read_to_string(&fixture_path).expect("Failed to read markets fixture");

    let markets: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse markets JSON");

    // markets.json是一个对象，key是symbol
    assert!(markets["BTC/USDT"].is_object() || markets.as_array().is_some());

    println!("✓ Markets fixture loaded successfully");
}

#[test]
fn test_load_balance_fixture() {
    let fixture_path = format!("{}/balances.json", FIXTURES_DIR);
    let fixture_content =
        fs::read_to_string(&fixture_path).expect("Failed to read balance fixture");

    let balance: Value =
        serde_json::from_str(&fixture_content).expect("Failed to parse balance JSON");

    // Verify balance structure
    assert!(balance["balances"].is_object() || balance["free"].is_object());

    println!("✓ Balance fixture loaded successfully");
}

// Example of a test that requires credentials
#[test]
fn test_with_binance_credentials() {
    let config = TestConfig::from_env().expect("Failed to load test config");

    // This will skip the test if no Binance credentials are available
    require_credentials!(config, binance);

    // At this point, we know we have Binance credentials
    let (api_key, api_secret) = config
        .get_active_api_key("binance")
        .expect("Should have Binance credentials");

    println!("✓ Test with Binance credentials");
    println!("  - API key length: {}", api_key.len());
    println!("  - Secret length: {}", api_secret.len());
    println!("  - Using testnet: {}", config.binance.use_testnet);
}
