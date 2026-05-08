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

use crate::support::{create_bybit, init_test};
use ccxt_core::types::common::ohlcv_request::OhlcvRequest;
use ccxt_core::{ExchangeConfig, types::Symbol};
use ccxt_exchanges::bybit::{Bybit, BybitBuilder};

/// Create Bybit client for public API tests.
fn create_public_bybit() -> Bybit {
    let config = init_test();
    create_bybit(&config).expect("Failed to create Bybit instance")
}

// ==================== Instance Creation Tests ====================

/// Test creating a new Bybit instance with default configuration.
#[test]
fn test_new_bybit_instance() {
    let config = ExchangeConfig {
        id: "bybit".to_string(),
        name: "Bybit".to_string(),
        ..Default::default()
    };

    let exchange = Bybit::new(config).unwrap();
    assert_eq!(exchange.id(), "bybit");
    assert_eq!(exchange.name(), "Bybit");
    assert_eq!(exchange.version(), "v5");
}

/// Test creating Bybit instance using builder pattern.
#[test]
fn test_bybit_builder() {
    let exchange = BybitBuilder::new()
        .testnet(false)
        .build()
        .expect("Failed to build Bybit");

    assert_eq!(exchange.id(), "bybit");
    assert_eq!(exchange.name(), "Bybit");
    assert!(!exchange.options().testnet);
}

/// Test creating Bybit instance with testnet mode.

/// Test Bybit timeframes.
#[test]
fn test_bybit_timeframes() {
    let exchange = BybitBuilder::new().build().unwrap();
    let timeframes = exchange.timeframes();

    assert!(timeframes.contains_key("1m"));
    assert!(timeframes.contains_key("5m"));
    assert!(timeframes.contains_key("1h"));
    assert!(timeframes.contains_key("1d"));
    assert_eq!(timeframes.len(), 13);
}

/// Test Bybit account type configuration.
#[test]
fn test_bybit_account_type() {
    let exchange = BybitBuilder::new()
        .account_type("SPOT")
        .build()
        .expect("Failed to build Bybit");

    assert_eq!(exchange.options().account_type, "SPOT");
}

/// Test Bybit recv_window configuration.
#[test]
fn test_bybit_recv_window() {
    let exchange = BybitBuilder::new()
        .recv_window(10000)
        .build()
        .expect("Failed to build Bybit");

    assert_eq!(exchange.options().recv_window, 10000);
}

// ==================== Public API Tests ====================

/// Test fetching all available markets from the real API.
#[tokio::test]
async fn test_public_markets() {
    let exchange = create_public_bybit();
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
    let exchange = create_public_bybit();

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
    // Note: Bybit Spot ticker API does NOT return bid/ask fields
    // See: https://bybit-exchange.github.io/docs/v5/websocket/public/ticker
    // For bid/ask, use fetch_order_book() or Linear/Swap markets instead
    assert!(ticker.high.is_some(), "High price should be present");
    assert!(ticker.low.is_some(), "Low price should be present");
}

/// Test fetching order book data from the real API.
#[tokio::test]
async fn test_public_orderbook() {
    let exchange = create_public_bybit();

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
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

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
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

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
    let exchange = BybitBuilder::new().build().expect("Failed to build Bybit");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_ticker("INVALID/SYMBOL").await;

    assert!(result.is_err(), "Should fail for invalid symbol");
}
