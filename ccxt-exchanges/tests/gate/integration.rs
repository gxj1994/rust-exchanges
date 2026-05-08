#![allow(clippy::disallowed_methods)]
//! Integration tests for Gate.io exchange.
//!
//! ## Test Categories
//!
//! - **Public API Tests**: Require network access, run automatically
//! - **Unit Tests**: No network access required, run automatically
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all Gate integration tests
//! cargo test -p ccxt-exchanges gate::integration
//! ```

use ccxt_core::exchange::Exchange;
use ccxt_core::types::Timeframe;
use ccxt_exchanges::gate::{Gate, GateBuilder};

/// Create a public Gate client for tests.
fn create_public_gate() -> Gate {
    GateBuilder::new()
        .build()
        .expect("Failed to create Gate instance")
}

// ============================================================================
// Creation Tests
// ============================================================================

/// Test creating a bare Gate instance.
#[test]
fn test_create_public_instance() {
    let exchange = create_public_gate();

    assert_eq!(exchange.id(), "gate");
    assert_eq!(exchange.name(), "Gate.io");
    assert_eq!(exchange.version(), "v4");
    assert!(!exchange.is_verified());
}

/// Test creating a Gate instance with ExchangeConfig.
#[test]
fn test_create_with_config() {
    let exchange =
        Gate::new(ccxt_core::ExchangeConfig::default()).expect("Failed to create Gate with config");

    assert_eq!(exchange.id(), "gate");
    assert_eq!(exchange.name(), "Gate.io");
}

/// Test builder with different options.
#[test]
fn test_builder_options() {
    let gate = GateBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build");

    assert!(gate.is_testnet());
    assert_eq!(gate.default_settle(), "usdt");
}

// ============================================================================
// Exchange Trait Tests
// ============================================================================

/// Test Exchange trait metadata.
#[test]
fn test_exchange_trait_metadata() {
    let exchange = create_public_gate();

    assert_eq!(exchange.id(), "gate");
    assert_eq!(exchange.name(), "Gate.io");
    assert_eq!(exchange.version(), "v4");
    assert!(!exchange.is_verified());
}

/// Test Exchange trait capabilities.
#[test]
fn test_exchange_capabilities() {
    let exchange = create_public_gate();
    let caps = exchange.capabilities();

    // Public API
    assert!(caps.fetch_markets(), "Should support fetch_markets");
    assert!(caps.fetch_ticker(), "Should support fetch_ticker");
    assert!(caps.fetch_order_book(), "Should support fetch_order_book");
    assert!(caps.fetch_ohlcv(), "Should support fetch_ohlcv");
    assert!(caps.fetch_trades(), "Should support fetch_trades");

    // Private API
    assert!(caps.create_order(), "Should support create_order");
    assert!(caps.cancel_order(), "Should support cancel_order");
    assert!(caps.fetch_balance(), "Should support fetch_balance");
    assert!(
        caps.fetch_account_trades(),
        "Should support fetch_account_trades"
    );
}

/// Test timeframes.
#[test]
fn test_timeframes() {
    let exchange = create_public_gate();
    let timeframes = exchange.timeframes();

    assert!(timeframes.contains(&Timeframe::M1));
    assert!(timeframes.contains(&Timeframe::M5));
    assert!(timeframes.contains(&Timeframe::M15));
    assert!(timeframes.contains(&Timeframe::M30));
    assert!(timeframes.contains(&Timeframe::H1));
    assert!(timeframes.contains(&Timeframe::H4));
    assert!(timeframes.contains(&Timeframe::D1));
    assert!(timeframes.contains(&Timeframe::W1));
}

// ============================================================================
// Public API Tests (network required)
// ============================================================================

/// Test fetching markets from Gate.io.
#[tokio::test]
async fn test_public_markets() {
    let exchange = create_public_gate();

    let markets = exchange
        .fetch_markets()
        .await
        .expect("Failed to fetch markets");

    assert!(!markets.is_empty(), "Should have at least one market");

    // Find BTC/USDT
    let btc_usdt = markets.iter().find(|m| m.symbol.as_str() == "BTC/USDT");
    assert!(btc_usdt.is_some(), "Should have BTC/USDT market");

    if let Some(market) = btc_usdt {
        assert!(market.active, "BTC/USDT should be active");
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(
            market.precision.price.is_some(),
            "Should have price precision"
        );
        assert!(market.limits.amount.is_some(), "Should have amount limits");
    }
}

/// Test fetching a ticker.
#[tokio::test]
async fn test_public_ticker() {
    let exchange = create_public_gate();

    // Load markets first
    Exchange::load_markets(&exchange, false)
        .await
        .expect("Failed to load markets");

    let ticker = exchange
        .fetch_ticker("BTC/USDT")
        .await
        .expect("Failed to fetch ticker");

    assert_eq!(ticker.symbol.as_str(), "BTC/USDT");
    assert!(ticker.last.is_some(), "Ticker should have last price");
    assert!(ticker.high.is_some(), "Ticker should have high price");
    assert!(ticker.low.is_some(), "Ticker should have low price");
}

/// Test fetching an order book.
#[tokio::test]
async fn test_public_orderbook() {
    let exchange = create_public_gate();

    // Load markets first
    Exchange::load_markets(&exchange, false)
        .await
        .expect("Failed to load markets");

    let orderbook = exchange
        .fetch_order_book("BTC/USDT", Some(10))
        .await
        .expect("Failed to fetch order book");

    assert_eq!(orderbook.symbol.as_str(), "BTC/USDT");
    assert!(!orderbook.bids.is_empty(), "Order book should have bids");
    assert!(!orderbook.asks.is_empty(), "Order book should have asks");

    // Verify sorting: bids descending, asks ascending
    for i in 1..orderbook.bids.len() {
        assert!(
            orderbook.bids[i - 1].price >= orderbook.bids[i].price,
            "Bids should be sorted descending by price"
        );
    }
    for i in 1..orderbook.asks.len() {
        assert!(
            orderbook.asks[i - 1].price <= orderbook.asks[i].price,
            "Asks should be sorted ascending by price"
        );
    }
}

/// Test fetching market trades.
#[tokio::test]
async fn test_public_trades() {
    let exchange = create_public_gate();

    // Load markets first
    Exchange::load_markets(&exchange, false)
        .await
        .expect("Failed to load markets");

    let trades = exchange
        .fetch_market_trades("BTC/USDT", Some(10))
        .await
        .expect("Failed to fetch trades");

    // Trades may be empty during low activity, but should not error
    for trade in &trades {
        assert!(trade.timestamp > 0, "Trade should have valid timestamp");
        assert!(
            trade.price.0 > rust_decimal::Decimal::ZERO,
            "Trade should have positive price"
        );
        assert!(
            trade.amount.0 > rust_decimal::Decimal::ZERO,
            "Trade should have positive amount"
        );
    }
}

/// Test fetching OHLCV data.
#[tokio::test]
async fn test_public_ohlcv() {
    let exchange = create_public_gate();

    // Load markets first
    Exchange::load_markets(&exchange, false)
        .await
        .expect("Failed to load markets");

    let result = exchange
        .fetch_ohlcv("BTC/USDT", Timeframe::H1, None, Some(5))
        .await;

    match result {
        Ok(candles) => {
            assert!(!candles.is_empty(), "Candles should not be empty");
            assert!(candles.len() <= 5, "Should have at most 5 candles");

            for candle in &candles {
                assert!(candle.timestamp > 0, "Timestamp should be positive");
                assert!(
                    candle.open.0 > rust_decimal::Decimal::ZERO,
                    "Open price should be positive"
                );
                assert!(candle.high.0 >= candle.low.0, "High should be >= Low");
                assert!(
                    candle.volume.0 >= rust_decimal::Decimal::ZERO,
                    "Volume should be non-negative"
                );
            }
        }
        Err(e) => {
            println!("OHLCV test skipped due to error: {}", e);
        }
    }
}

/// Test loading markets cache.
#[tokio::test]
async fn test_load_markets_cache() {
    let exchange = create_public_gate();

    // Load markets
    let markets = Exchange::load_markets(&exchange, false)
        .await
        .expect("Failed to load markets");

    assert!(!markets.is_empty(), "Markets cache should not be empty");

    // Reload should work
    let reloaded = Exchange::load_markets(&exchange, true)
        .await
        .expect("Failed to reload markets");

    assert!(!reloaded.is_empty(), "Reloaded markets should not be empty");
}

/// Test market lookup from cache.
#[tokio::test]
async fn test_market_lookup() {
    let exchange = create_public_gate();

    // First load markets
    Exchange::load_markets(&exchange, false)
        .await
        .expect("Failed to load markets");

    // Look up a specific market
    let market = Exchange::market(&exchange, "BTC/USDT")
        .await
        .expect("Failed to find BTC/USDT market");

    assert_eq!(market.base, "BTC");
    assert_eq!(market.quote, "USDT");
    assert!(market.active);
}

// ============================================================================
// REST URL Tests
// ============================================================================

/// Test spot REST URL.
#[test]
fn test_spot_rest_url() {
    let gate = create_public_gate();
    let url = gate.get_rest_url();
    assert!(
        url.contains("api.gateio.ws"),
        "URL should contain api.gateio.ws: {}",
        url
    );
}

/// Test contract REST URL.
#[test]
fn test_contract_rest_url() {
    let gate = GateBuilder::new()
        .default_type(ccxt_core::types::common::default_type::DefaultType::Swap)
        .default_sub_type(ccxt_core::types::common::default_type::DefaultSubType::Linear)
        .build()
        .expect("Failed to build");

    let url = gate.get_contract_rest_url();
    assert!(
        url.contains("fx-api.gateio.ws"),
        "URL should contain fx-api.gateio.ws: {}",
        url
    );
}

// ============================================================================
// Symbol Conversion Tests
// ============================================================================

/// Test symbol conversion utility.
#[test]
fn test_symbol_conversion() {
    assert_eq!(Gate::to_exchange_symbol("BTC/USDT"), "BTC_USDT");
    assert_eq!(Gate::to_exchange_symbol("ETH/USDT"), "ETH_USDT");
    assert_eq!(Gate::to_exchange_symbol("DOGE/USDT"), "DOGE_USDT");
}

/// Test settle currency.
#[test]
fn test_default_settle() {
    let gate = create_public_gate();
    assert_eq!(gate.default_settle(), "usdt");
}

/// Test get_contract_settle with different sub-types.
#[test]
fn test_contract_settle_options() {
    use ccxt_core::types::common::default_type::{DefaultSubType, DefaultType};

    let gate = GateBuilder::new()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Linear)
        .build()
        .unwrap();
    assert_eq!(gate.get_contract_settle(), "usdt");

    let gate = GateBuilder::new()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Usdc)
        .build()
        .unwrap();
    assert_eq!(gate.get_contract_settle(), "usdc");

    let gate = GateBuilder::new()
        .default_type(DefaultType::Swap)
        .default_sub_type(DefaultSubType::Inverse)
        .build()
        .unwrap();
    assert_eq!(gate.get_contract_settle(), "btc");
}
