#![allow(clippy::disallowed_methods)]
//! Integration tests for HyperLiquid exchange.
//!
//! ## Test Categories
//!
//! - **Unit Tests**: No network access required, run automatically
//! - **Public API Tests**: Require network access, run automatically
//! - **Private API Tests**: Require HYPERLIQUID_PRIVATE_KEY, automatically skipped if not configured
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all integration tests
//! cargo test -p ccxt-exchanges --test hyperliquid_integration_test
//! ```

use ccxt_core::{exchange::Exchange, types::Symbol};
use ccxt_exchanges::hyperliquid::{HyperLiquid, HyperLiquidBuilder};

/// Create HyperLiquid client for public API tests (testnet).
fn create_public_hyperliquid() -> HyperLiquid {
    HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to create HyperLiquid instance")
}

/// Test creating a HyperLiquid instance without authentication.
#[test]
fn test_create_public_instance() {
    let exchange = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    assert_eq!(exchange.id(), "hyperliquid");
    assert_eq!(exchange.name(), "HyperLiquid");
    assert!(exchange.options().testnet);
    assert!(exchange.auth().is_none());
}

/// Test creating a HyperLiquid instance with authentication.
#[test]
fn test_create_authenticated_instance() {
    // Test private key (DO NOT USE IN PRODUCTION)
    let test_key = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    let exchange = HyperLiquidBuilder::new()
        .private_key(test_key)
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    assert!(exchange.auth().is_some());
    assert!(exchange.wallet_address().is_some());
}

/// Test that invalid private key is rejected.
#[test]
fn test_invalid_private_key_rejected() {
    let result = HyperLiquidBuilder::new()
        .private_key("invalid_key")
        .testnet(true)
        .build();

    assert!(result.is_err());
}

/// Test Exchange trait capabilities.
#[test]
fn test_exchange_capabilities() {
    let exchange = HyperLiquidBuilder::new()
        .testnet(true)
        .build()
        .expect("Failed to build HyperLiquid");

    let caps = exchange.capabilities();

    // Public API
    assert!(caps.fetch_markets());
    assert!(caps.fetch_ticker());
    assert!(caps.fetch_tickers());
    assert!(caps.fetch_order_book());
    assert!(caps.fetch_trades());
    assert!(caps.fetch_ohlcv());

    // Private API
    assert!(caps.create_order());
    assert!(caps.cancel_order());
    assert!(caps.fetch_open_orders());
    assert!(caps.fetch_balance());
    assert!(caps.fetch_positions());
    assert!(caps.set_leverage());

    // Not supported
    assert!(!caps.fetch_currencies());
    assert!(!caps.fetch_account_trades());
}

/// Test fetching markets from testnet.
#[tokio::test]
async fn test_public_markets() {
    let exchange = create_public_hyperliquid();

    let markets = exchange
        .fetch_markets()
        .await
        .expect("Failed to fetch markets");

    assert!(!markets.is_empty(), "Should have at least one market");

    for market in markets.values() {
        // All HyperLiquid markets should be perpetual contracts
        assert!(
            market.symbol.ends_with("/USDC:USDC"),
            "Symbol should end with /USDC:USDC"
        );
        assert!(market.active, "Market should be active");
        assert!(market.margin, "Market should support margin");
        assert_eq!(market.contract, Some(true), "Market should be a contract");
    }
}

/// Test fetching ticker from testnet.
#[tokio::test]
async fn test_public_ticker() {
    let exchange = create_public_hyperliquid();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch BTC ticker
    let ticker = exchange
        .fetch_ticker("BTC/USDC:USDC")
        .await
        .expect("Failed to fetch ticker");

    assert_eq!(ticker.symbol, Symbol::new_unchecked("BTC/USDC:USDC"));
    assert!(ticker.last.is_some(), "Ticker should have last price");
    assert!(ticker.timestamp > 0, "Ticker should have valid timestamp");
}

/// Test fetching order book from testnet.
#[tokio::test]
async fn test_public_orderbook() {
    let exchange = create_public_hyperliquid();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch BTC order book
    let orderbook = exchange
        .fetch_order_book("BTC/USDC:USDC", None)
        .await
        .expect("Failed to fetch order book");

    assert_eq!(orderbook.symbol, Symbol::new_unchecked("BTC/USDC:USDC"));
    assert!(
        !orderbook.bids.is_empty() || !orderbook.asks.is_empty(),
        "Order book should have entries"
    );

    // Verify sorting
    for i in 1..orderbook.bids.len() {
        assert!(
            orderbook.bids[i - 1].price >= orderbook.bids[i].price,
            "Bids should be sorted descending"
        );
    }
    for i in 1..orderbook.asks.len() {
        assert!(
            orderbook.asks[i - 1].price <= orderbook.asks[i].price,
            "Asks should be sorted ascending"
        );
    }
}

/// Test fetching trades from testnet.
#[tokio::test]
async fn test_public_trades() {
    let exchange = create_public_hyperliquid();

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch BTC trades
    let trades = exchange
        .fetch_market_trades("BTC/USDC:USDC", Some(10))
        .await
        .expect("Failed to fetch trades");

    // Trades may be empty on testnet, but should not error
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

/// Test fetching OHLCV (candlestick) data from mainnet.
/// Note: Uses mainnet because testnet OHLCV endpoint may have limited support.
#[tokio::test]
async fn test_public_ohlcv() {
    // Use mainnet for OHLCV as testnet may not support this endpoint well
    let exchange = HyperLiquidBuilder::new()
        .testnet(false)
        .build()
        .expect("Failed to create HyperLiquid instance");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange
        .fetch_ohlcv("BTC/USDC:USDC", "1h", None, Some(5))
        .await;

    match result {
        Ok(candles) => {
            assert!(!candles.is_empty(), "Candles should not be empty");
            assert!(candles.len() <= 5, "Should have at most 5 candles");

            use rust_decimal::Decimal;
            for candle in &candles {
                assert!(candle.timestamp > 0, "Timestamp should be positive");
                assert!(
                    candle.open.0 > Decimal::ZERO,
                    "Open price should be positive"
                );
                assert!(candle.high.0 >= candle.low.0, "High should be >= low");
                assert!(candle.high.0 >= candle.open.0, "High should be >= open");
                assert!(candle.high.0 >= candle.close.0, "High should be >= close");
                assert!(candle.low.0 <= candle.open.0, "Low should be <= open");
                assert!(candle.low.0 <= candle.close.0, "Low should be <= close");
                assert!(
                    candle.volume.0 >= Decimal::ZERO,
                    "Volume should be non-negative"
                );
            }
        }
        Err(e) => {
            // Log error but don't fail - OHLCV endpoint may be temporarily unavailable
            println!("OHLCV test skipped due to error: {}", e);
        }
    }
}

/// Test fetching spot markets.
#[tokio::test]
async fn test_fetch_spot_markets() {
    let exchange = create_public_hyperliquid();

    // Fetch spot markets
    match exchange.fetch_spot_markets().await {
        Ok(markets) => {
            assert!(!markets.is_empty(), "Spot markets should not be empty");

            // Log first few spot markets
            println!("\n=== HyperLiquid Spot Markets ===");
            let count = std::cmp::min(5, markets.len());
            for (i, (symbol, market)) in markets.iter().take(count).enumerate() {
                println!(
                    "{}: {} (id={}, base={}, quote={}, type={})",
                    i, symbol, market.id, market.base, market.quote, market.market_type
                );

                // Verify asset_index format (should be >= 10000)
                let asset_index: u64 = market.id.parse().unwrap_or(0);
                assert!(
                    asset_index >= 10000,
                    "Spot asset_index should be >= 10000, got {}",
                    asset_index
                );
            }
            println!("Total spot markets: {}", markets.len());
        }
        Err(e) => {
            // Spot markets may not be available on testnet
            println!("Spot markets test skipped: {}", e);
        }
    }
}

/// Test spot market symbol format.
#[tokio::test]
async fn test_spot_symbol_format() {
    use ccxt_exchanges::hyperliquid::core::symbol::HyperliquidSymbolConverter;

    // Test PURR/USDC spot symbol
    let spot_coin = "PURR/USDC";
    assert!(HyperliquidSymbolConverter::is_spot_coin(spot_coin));
    assert_eq!(
        HyperliquidSymbolConverter::coin_to_unified(spot_coin),
        "PURR/USDC"
    );

    // Test @107 spot symbol
    let spot_index = "@107";
    assert!(HyperliquidSymbolConverter::is_spot_coin(spot_index));
    assert_eq!(
        HyperliquidSymbolConverter::coin_to_unified(spot_index),
        "@107"
    );

    // Test BTC perpetual symbol (should not be spot)
    let perp_coin = "BTC";
    assert!(!HyperliquidSymbolConverter::is_spot_coin(perp_coin));
    assert_eq!(
        HyperliquidSymbolConverter::coin_to_unified(perp_coin),
        "BTC/USDC:USDC"
    );
}
