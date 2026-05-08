//! Bybit Integration Tests
//!
//! These tests verify the Bybit exchange implementation against the real API.
//! They can be run with: cargo test --test bybit_integration_test
//!
//! ## Test Categories
//!
//! - **Public API Tests**: No credentials required, run automatically
//! - **Private API Tests**: Require credentials, automatically skipped if not configured
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all integration tests
//! cargo test -p ccxt-exchanges --test bybit_integration_test
//!
//! # Run only public API tests
//! cargo test -p ccxt-exchanges --test bybit_integration_test public
//! ```

use crate::support::{create_bybit_with_credentials, init_test, should_skip_private_tests};
use ccxt_exchanges::bybit::{Bybit, BybitBuilder};

/// Create Bybit client for private API tests.
/// Returns None if credentials are not configured.
fn create_private_bybit() -> Option<Bybit> {
    let config = init_test();
    if should_skip_private_tests("bybit") {
        return None;
    }
    create_bybit_with_credentials(&config, "spot").ok()
}

// ==================== Private API Tests ====================

/// Test fetching account balance.
///
/// Note: Requires API credentials.
#[tokio::test]
async fn test_private_balance() {
    let bybit = create_private_bybit();
    if bybit.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }
    let exchange = bybit.unwrap();

    let result = exchange.fetch_balance().await;

    assert!(
        result.is_ok(),
        "Failed to fetch balance: {:?}",
        result.err()
    );
}

/// Test fetching open orders.
///
/// Note: Requires API credentials.
#[tokio::test]
async fn test_private_orders() {
    let bybit = create_private_bybit();
    if bybit.is_none() {
        println!("⚠️  Skip test: API credentials not set");
        return;
    }
    let exchange = bybit.unwrap();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await;

    assert!(
        result.is_ok(),
        "Failed to fetch open orders: {:?}",
        result.err()
    );
}

/// Test that operations requiring credentials fail without API keys.
#[tokio::test]
async fn test_edge_no_credentials() {
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

    let result = exchange.fetch_balance().await;
    assert!(result.is_err(), "Should fail without API credentials");
}

/// Test invalid API key handling.
#[tokio::test]
async fn test_edge_invalid_api_key() {
    let exchange = BybitBuilder::new()
        .api_key("invalid_key")
        .secret("invalid_secret")
        .build()
        .expect("Failed to build Bybit");

    let result = exchange.fetch_balance().await;
    assert!(result.is_err(), "Should fail with invalid API key");
}
