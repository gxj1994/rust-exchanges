//! Bitget fetch_trades integration tests
//!
//! Tests for the public trades endpoint:
//! - GET /api/v3/market/fills
//!
//! ## Prerequisites
//!
//! - No API credentials required (public endpoint)
//!
//! ## API Documentation
//!
//! - https://www.bitgetapp.com/zh-CN/api-doc/uta/public/Fills

use crate::support::{create_bitget, init_test};
use ccxt_core::traits::Margin;
use ccxt_core::types::{
    OhlcvRequest, Symbol,
    financial::{Amount, Price},
};
use ccxt_exchanges::bitget::BitgetBuilder;

// ==================== Instance Creation Tests ====================

/// Test creating a new Bitget instance with default configuration.
#[test]
fn test_new_bitget_instance() {
    let exchange = BitgetBuilder::new()
        .build()
        .expect("Failed to build Bitget");

    assert_eq!(exchange.id(), "bitget");
    assert_eq!(exchange.name(), "Bitget");
    assert_eq!(exchange.version(), "v2");
}

/// Test creating Bitget instance using builder pattern.
#[test]
fn test_bitget_builder() {
    let exchange = BitgetBuilder::new()
        .sandbox(false)
        .build()
        .expect("Failed to build Bitget");

    assert_eq!(exchange.id(), "bitget");
    assert_eq!(exchange.name(), "Bitget");
    assert!(!exchange.options().testnet);
}

/// Test Bitget timeframes.
#[test]
fn test_bitget_timeframes() {
    let exchange = BitgetBuilder::new().build().unwrap();
    let timeframes = exchange.timeframes();

    assert!(timeframes.contains_key("1m"));
    assert!(timeframes.contains_key("5m"));
    assert!(timeframes.contains_key("1h"));
    assert!(timeframes.contains_key("1d"));
    assert!(timeframes.contains_key("3d"));
    assert_eq!(timeframes.len(), 13);
}

/// Test Bitget product type configuration.
#[test]
fn test_bitget_product_type() {
    let exchange = BitgetBuilder::new()
        .product_type("spot")
        .build()
        .expect("Failed to build Bitget");

    assert_eq!(exchange.options().product_type, "spot");
}

/// Test Bitget recv_window configuration.
#[test]
fn test_bitget_recv_window() {
    let exchange = BitgetBuilder::new()
        .recv_window(10000)
        .build()
        .expect("Failed to build Bitget");

    assert_eq!(exchange.options().recv_window, 10000);
}

// ==================== Public API Tests ====================

/// Test fetching all available markets from real API.
#[tokio::test]
async fn test_public_markets() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");
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

/// Test fetching ticker data for BTC/USDT from real API.
#[tokio::test]
async fn test_public_ticker() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

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

/// Test fetching order book data from real API.
#[tokio::test]
async fn test_public_orderbook() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

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

/// Test fetching OHLCV (candlestick) data from real API.
#[tokio::test]
async fn test_public_ohlcv() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

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

/// Test fetching current funding rate for a swap symbol.
#[tokio::test]
async fn test_public_funding_rate() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Test with USDT perpetual swap
    let result = exchange.fetch_funding_rate("BTC/USDT:USDT").await;

    assert!(
        result.is_ok(),
        "Failed to fetch funding rate: {:?}",
        result.err()
    );
    let funding_rate = result.unwrap();

    assert_eq!(funding_rate.symbol, "BTC/USDT:USDT".to_string());
    assert!(
        funding_rate.funding_rate.is_some(),
        "Funding rate should be present"
    );
    assert!(
        funding_rate.funding_timestamp.is_some(),
        "Funding timestamp should be present"
    );

    // Funding rate should be a reasonable value (typically between -0.003 and 0.003)
    if let Some(rate) = funding_rate.funding_rate {
        assert!(
            rate >= -0.01 && rate <= 0.01,
            "Funding rate {} seems unreasonable (expected between -1% and 1%)",
            rate
        );
    }
}

/// Test fetching funding rate history for a swap symbol.
#[tokio::test]
async fn test_public_funding_rate_history() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Test with USDT perpetual swap, fetch last 10 records
    let result = exchange
        .fetch_funding_rate_history("BTC/USDT:USDT", None, Some(10))
        .await;

    assert!(
        result.is_ok(),
        "Failed to fetch funding rate history: {:?}",
        result.err()
    );
    let history = result.unwrap();

    assert!(!history.is_empty(), "History should not be empty");
    assert!(
        history.len() <= 10,
        "Should have at most 10 history records"
    );

    for record in &history {
        assert_eq!(record.symbol, "BTC/USDT:USDT".to_string());
        assert!(
            record.funding_rate.is_some(),
            "Funding rate should be present"
        );
        assert!(record.timestamp.is_some(), "Timestamp should be present");

        // Funding rate should be a reasonable value
        if let Some(rate) = record.funding_rate {
            assert!(
                rate >= -0.01 && rate <= 0.01,
                "Funding rate {} seems unreasonable",
                rate
            );
        }
    }

    // Verify timestamps are in descending order (newest first)
    for i in 1..history.len() {
        if let (Some(ts1), Some(ts2)) = (history[i - 1].timestamp, history[i].timestamp) {
            assert!(
                ts1 >= ts2,
                "History should be sorted by timestamp descending"
            );
        }
    }
}

/// Test error handling for invalid trading symbols.
#[tokio::test]
async fn test_edge_invalid_symbol() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

    // First load markets
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let result = exchange.fetch_ticker("INVALID/SYMBOL").await;

    assert!(result.is_err(), "Should fail for invalid symbol");
}

/// Test: Fetch recent trades for BTC/USDT
#[tokio::test]
async fn test_fetch_trades_basic() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

    // Load markets first
    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch recent trades
    let trades = exchange
        .fetch_market_trades("BTC/USDT", Some(10))
        .await
        .expect("Failed to fetch trades");

    // Should return some trades
    assert!(!trades.is_empty(), "Should return at least one trade");
    assert!(
        trades.len() <= 10,
        "Should not return more than requested limit"
    );

    // Verify trade structure
    for trade in &trades {
        assert!(
            !trade.symbol.as_str().is_empty(),
            "Trade should have symbol"
        );
        assert!(
            trade.price > Price::new(rust_decimal::Decimal::ZERO),
            "Trade price should be positive"
        );
        assert!(
            trade.amount > Amount::new(rust_decimal::Decimal::ZERO),
            "Trade amount should be positive"
        );
        assert!(trade.timestamp > 0, "Trade should have timestamp");
        // Side should be either buy or sell
        assert!(
            trade.side == ccxt_core::types::OrderSide::Buy
                || trade.side == ccxt_core::types::OrderSide::Sell,
            "Trade side should be Buy or Sell"
        );
    }

    println!("✅ Fetched {} trades for BTC/USDT", trades.len());
    println!("   First trade:");
    if let Some(first) = trades.first() {
        println!("     ID: {:?}", first.id);
        println!("     Price: {}", first.price);
        println!("     Amount: {}", first.amount);
        println!("     Side: {:?}", first.side);
        println!("     Timestamp: {}", first.timestamp);
    }
}

/// Test: Fetch trades with default limit
#[tokio::test]
async fn test_fetch_trades_default_limit() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    // Fetch with default limit (should be 100)
    let trades = exchange
        .fetch_market_trades("BTC/USDT", None)
        .await
        .expect("Failed to fetch trades");

    // Default limit is 100
    assert!(trades.len() <= 100, "Default limit should not exceed 100");

    println!("✅ Fetched {} trades with default limit", trades.len());
}

/// Test: Fetch trades for different symbols
#[tokio::test]
async fn test_fetch_trades_multiple_symbols() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let symbols = vec!["BTC/USDT", "ETH/USDT"];

    for symbol in symbols {
        let trades = exchange
            .fetch_market_trades(symbol, Some(5))
            .await
            .expect(&format!("Failed to fetch trades for {}", symbol));

        assert!(!trades.is_empty(), "Should return trades for {}", symbol);

        // All trades should be for the correct symbol
        for trade in &trades {
            assert_eq!(trade.symbol.as_str(), symbol);
        }

        println!("✅ Fetched {} trades for {}", trades.len(), symbol);
    }
}

/// Test: Verify trade timestamp ordering (newest first)
#[tokio::test]
async fn test_fetch_trades_timestamp_order() {
    let config = init_test();
    let exchange = create_bitget(&config).expect("Failed to create Bitget");

    exchange
        .load_markets(false)
        .await
        .expect("Failed to load markets");

    let trades = exchange
        .fetch_market_trades("BTC/USDT", Some(20))
        .await
        .expect("Failed to fetch trades");

    if trades.len() > 1 {
        // Trades should be in descending order (newest first)
        for i in 0..trades.len() - 1 {
            assert!(
                trades[i].timestamp >= trades[i + 1].timestamp,
                "Trades should be sorted by timestamp (newest first)"
            );
        }
        println!("✅ Trades are correctly sorted by timestamp (newest first)");
    }
}
