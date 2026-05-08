//! OKX Integration Tests
//!
//! These tests verify the OKX exchange implementation against the real API.
//! They can be run with: cargo test --test okx_integration_test
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
//! cargo test -p ccxt-exchanges --test okx_integration_test
//!
//! # Run only public API tests
//! cargo test -p ccxt-exchanges --test okx_integration_test public
//! ```

use crate::support::{
    create_okx, create_okx_with_credentials, init_test, should_skip_private_tests,
};
use ccxt_core::types::common::ohlcv_request::OhlcvRequest;
use ccxt_core::{ExchangeConfig, types::Symbol};
use ccxt_exchanges::okx::{Okx, OkxBuilder};

/// Create OKX client for public API tests.
fn create_public_okx() -> Okx {
    let config = init_test();
    create_okx(&config).expect("Failed to create OKX instance")
}

/// Create OKX client for private API tests.
/// Returns None if credentials are not configured.
fn create_private_okx() -> Option<Okx> {
    let config = init_test();
    if should_skip_private_tests("okx") {
        return None;
    }
    create_okx_with_credentials(&config, "spot").ok()
}

/// Macro to skip test if no credentials available
macro_rules! skip_if_no_credentials {
    ($client:expr) => {
        if $client.is_none() {
            println!("SKIPPED: No OKX credentials configured");
            return;
        }
    };
}

// ==================== Instance Creation Tests ====================

/// Test creating a new OKX instance with default configuration.
#[test]
fn test_new_okx_instance() {
    let config = ExchangeConfig {
        id: "okx".to_string(),
        name: "OKX".to_string(),
        ..Default::default()
    };

    let exchange = Okx::new(config).unwrap();
    assert_eq!(exchange.id(), "okx");
    assert_eq!(exchange.name(), "OKX");
    assert_eq!(exchange.version(), "v5");
}

/// Test creating OKX instance using builder pattern.
#[test]
fn test_okx_builder() {
    let exchange = OkxBuilder::new()
        .sandbox(false)
        .build()
        .expect("Failed to build OKX");

    assert_eq!(exchange.id(), "okx");
    assert_eq!(exchange.name(), "OKX");
    assert!(!exchange.options().testnet);
}

/// Test OKX timeframes.
#[test]
fn test_okx_timeframes() {
    let exchange = OkxBuilder::new().build().unwrap();
    let timeframes = exchange.timeframes();

    assert!(timeframes.contains_key("1m"));
    assert!(timeframes.contains_key("5m"));
    assert!(timeframes.contains_key("1h"));
    assert!(timeframes.contains_key("1d"));
    assert_eq!(timeframes.len(), 13);
}

// ==================== Public API Tests ====================

/// Test fetching all available markets from the real API.
#[tokio::test]
async fn test_public_markets() {
    let exchange = create_public_okx();
    let result = exchange.fetch_markets().await;

    assert!(
        result.is_ok(),
        "Failed to fetch markets: {:?}",
        result.err()
    );
    let markets = result.unwrap();

    assert!(
        markets.len() > 10,
        "Expected more than 10 markets, got {}",
        markets.len()
    );

    // Check for common trading pair
    let btc_usdt = markets
        .values()
        .find(|m| m.symbol == Symbol::new_unchecked("BTC/USDT"));
    assert!(btc_usdt.is_some(), "BTC/USDT market not found");

    if let Some(market) = btc_usdt {
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
    }
}

/// Test fetching ticker data for BTC/USDT from the real API.
#[tokio::test]
async fn test_public_ticker() {
    let exchange = create_public_okx();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_ticker("BTC/USDT").await;

    assert!(result.is_ok(), "Failed to fetch ticker: {:?}", result.err());
    let ticker = result.unwrap();

    assert_eq!(ticker.symbol, Symbol::new_unchecked("BTC/USDT"));
    assert!(ticker.last.is_some(), "Last price should be present");
    assert!(ticker.bid.is_some(), "Bid price should be present");
    assert!(ticker.ask.is_some(), "Ask price should be present");
    assert!(ticker.high.is_some(), "High price should be present");
    assert!(ticker.low.is_some(), "Low price should be present");
}

/// Test fetching order book data from the real API.
#[tokio::test]
async fn test_public_orderbook() {
    let exchange = create_public_okx();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_order_book("BTC/USDT", Some(10)).await;

    assert!(
        result.is_ok(),
        "Failed to fetch order book: {:?}",
        result.err()
    );
    let order_book = result.unwrap();

    assert_eq!(order_book.symbol, Symbol::new_unchecked("BTC/USDT"));
    assert!(!order_book.bids.is_empty(), "Bids should not be empty");
    assert!(!order_book.asks.is_empty(), "Asks should not be empty");
    assert!(order_book.bids.len() <= 10, "Should have at most 10 bids");
    assert!(order_book.asks.len() <= 10, "Should have at most 10 asks");

    // Verify sorting: bids descending, asks ascending
    for i in 1..order_book.bids.len() {
        assert!(
            order_book.bids[i - 1].price >= order_book.bids[i].price,
            "Bids should be sorted descending"
        );
    }
    for i in 1..order_book.asks.len() {
        assert!(
            order_book.asks[i - 1].price <= order_book.asks[i].price,
            "Asks should be sorted ascending"
        );
    }

    if let (Some(best_bid), Some(best_ask)) = (order_book.bids.first(), order_book.asks.first()) {
        assert!(
            best_bid.price < best_ask.price,
            "Best bid ({}) should be less than best ask ({})",
            best_bid.price,
            best_ask.price
        );
    }
}

/// Test fetching recent trades from the real API.
#[tokio::test]
async fn test_public_trades() {
    let exchange = create_public_okx();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_market_trades("BTC/USDT", Some(5)).await;

    assert!(result.is_ok(), "Failed to fetch trades: {:?}", result.err());
    let trades = result.unwrap();

    assert!(!trades.is_empty(), "Trades should not be empty");
    assert!(trades.len() <= 5, "Should have at most 5 trades");

    for trade in &trades {
        assert_eq!(trade.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert!(
            trade.price.0 > rust_decimal::Decimal::ZERO,
            "Trade price should be positive"
        );
        assert!(
            trade.amount.0 > rust_decimal::Decimal::ZERO,
            "Trade amount should be positive"
        );
        assert!(trade.timestamp > 0, "Trade should have timestamp");
    }
}

/// Test fetching OHLCV (candlestick) data from the real API.
#[tokio::test]
async fn test_public_ohlcv() {
    let exchange = create_public_okx();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let request = OhlcvRequest::builder()
        .symbol("BTC/USDT")
        .timeframe("1h")
        .limit(5)
        .build()
        .expect("Failed to build OHLCV request");
    let result = exchange.fetch_ohlcv(request).await;

    assert!(result.is_ok(), "Failed to fetch OHLCV: {:?}", result.err());
    let candles = result.unwrap();

    assert!(!candles.is_empty(), "Candles should not be empty");
    assert!(candles.len() <= 5, "Should have at most 5 candles");

    for candle in &candles {
        assert!(candle.open > 0.0, "Open price should be positive");
        assert!(candle.high >= candle.low, "High should be >= low");
        assert!(candle.high >= candle.open, "High should be >= open");
        assert!(candle.high >= candle.close, "High should be >= close");
        assert!(candle.low <= candle.open, "Low should be <= open");
        assert!(candle.low <= candle.close, "Low should be <= close");
        assert!(candle.volume >= 0.0, "Volume should be non-negative");
    }
}

/// Test error handling for invalid trading symbols.
#[tokio::test]
async fn test_edge_invalid_symbol() {
    let exchange = create_public_okx();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_ticker("INVALID/SYMBOL").await;

    assert!(result.is_err(), "Should fail for invalid symbol");
}

// ==================== Private API Tests ====================

/// Test fetching account balance.
///
/// Note: Requires API credentials. Automatically skipped if not configured.
#[tokio::test]
async fn test_private_balance() {
    let exchange = create_private_okx();
    skip_if_no_credentials!(exchange);
    let exchange = exchange.unwrap();

    let result = exchange.fetch_balance().await;

    assert!(
        result.is_ok(),
        "Failed to fetch balance: {:?}",
        result.err()
    );
}

/// Test fetching open orders.
///
/// Note: Requires API credentials. Automatically skipped if not configured.
#[tokio::test]
async fn test_private_orders() {
    let exchange = create_private_okx();
    skip_if_no_credentials!(exchange);
    let exchange = exchange.unwrap();

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
    let exchange = create_public_okx();

    let result = exchange.fetch_balance().await;
    assert!(result.is_err(), "Should fail without API credentials");
}

/// Test invalid API key handling.
#[tokio::test]
async fn test_edge_invalid_api_key() {
    let exchange = OkxBuilder::new()
        .api_key("invalid_key")
        .secret("invalid_secret")
        .passphrase("invalid_passphrase")
        .build()
        .expect("Failed to build OKX");

    let result = exchange.fetch_balance().await;
    assert!(result.is_err(), "Should fail with invalid API key");
}
