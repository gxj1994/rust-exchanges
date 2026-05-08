//! Basic usage example for CCXT Rust
//!
//! This example demonstrates creating basic data structures
//! and working with the type system. Updated to showcase
//! i64 timestamp usage throughout the library.

#![allow(clippy::disallowed_methods)]

use anyhow::Result;
use ccxt_core::prelude::*;
use rust_decimal_macros::dec;

// Include common logging macros
#[path = "common/mod.rs"]
mod common;

fn main() -> Result<()> {
    log_title!("CCXT Rust Basic Usage");

    // Create a spot market
    log_section!("Market Creation");
    let market = Market::new_spot(
        "btcusdt".to_string(),
        Symbol::new_unchecked("BTC/USDT"),
        "BTC".to_string(),
        "USDT".to_string(),
    );
    log_success!("Created spot market");
    log_field!("Symbol", market.symbol);
    log_field!("Type", format!("{:?}", market.market_type));
    log_field!("Base", market.base);
    log_field!("Quote", market.quote);

    // Create a limit buy order
    log_section!("Order Creation");
    let order = Order::new(
        "order-123".to_string(),
        Symbol::new_unchecked("BTC/USDT"),
        OrderType::Limit,
        OrderSide::Buy,
        dec!(0.1),
        Some(dec!(50000.0)),
        OrderStatus::Open,
    );
    log_success!("Created limit buy order");
    log_field!("ID", order.id);
    log_field!("Symbol", order.symbol);
    log_field!("Type", format!("{:?}", order.order_type));
    log_field!("Side", format!("{:?}", order.side));
    log_field!("Price", format!("{:?}", order.price));
    log_field!("Amount", order.amount);
    log_field!("Status", format!("{:?}", order.status));

    // Create a ticker with i64 timestamp
    log_section!("Ticker Creation");
    let timestamp_ms: i64 = chrono::Utc::now().timestamp_millis();
    let mut ticker = Ticker::new(Symbol::new_unchecked("BTC/USDT"), timestamp_ms);
    ticker.bid = Some(dec!(49950.0).into());
    ticker.ask = Some(dec!(50050.0).into());
    ticker.last = Some(dec!(50000.0).into());
    ticker.base_volume = Some(dec!(1234.5).into());

    log_success!("Created ticker");
    log_field!("Symbol", ticker.symbol);
    log_field!("Bid", format!("{:?}", ticker.bid));
    log_field!("Ask", format!("{:?}", ticker.ask));
    log_field!("Last", format!("{:?}", ticker.last));
    if let Some(spread) = ticker.spread() {
        log_field!("Spread", spread);
    }

    // Create an order book with i64 timestamp
    log_section!("Order Book Creation");
    let timestamp_ms: i64 = chrono::Utc::now().timestamp_millis();
    let mut orderbook = OrderBook::new(Symbol::new_unchecked("BTC/USDT"), timestamp_ms);

    orderbook.bids = vec![
        OrderBookEntry::new(dec!(50000.0).into(), dec!(1.0).into()),
        OrderBookEntry::new(dec!(49900.0).into(), dec!(2.0).into()),
        OrderBookEntry::new(dec!(49800.0).into(), dec!(1.5).into()),
    ];

    orderbook.asks = vec![
        OrderBookEntry::new(dec!(50100.0).into(), dec!(1.0).into()),
        OrderBookEntry::new(dec!(50200.0).into(), dec!(2.0).into()),
        OrderBookEntry::new(dec!(50300.0).into(), dec!(1.5).into()),
    ];

    log_success!("Created order book");
    log_field!("Symbol", orderbook.symbol);
    if let Some(best_bid) = orderbook.best_bid() {
        log_field!(
            "Best Bid",
            format!("{} @ {}", best_bid.amount, best_bid.price)
        );
    }
    if let Some(best_ask) = orderbook.best_ask() {
        log_field!(
            "Best Ask",
            format!("{} @ {}", best_ask.amount, best_ask.price)
        );
    }
    if let Some(spread) = orderbook.spread() {
        log_field!("Spread", spread);
    }
    log_field!("Total Bid Volume", orderbook.bid_volume());
    log_field!("Total Ask Volume", orderbook.ask_volume());

    // Create a trade with i64 timestamp
    log_section!("Trade Creation");
    let timestamp_ms: i64 = chrono::Utc::now().timestamp_millis();
    let trade = Trade::new(
        Symbol::new_unchecked("BTC/USDT"),
        OrderSide::Buy,
        dec!(50000.0).into(),
        dec!(0.5).into(),
        timestamp_ms,
    );
    log_success!("Created trade");
    log_field!("Symbol", trade.symbol);
    log_field!("Side", format!("{:?}", trade.side));
    log_field!("Price", trade.price);
    log_field!("Amount", trade.amount);
    log_field!("Cost", format!("{:?}", trade.cost));

    // Create OHLCV data with i64 timestamp
    log_section!("OHLCV Creation");
    let timestamp_ms: i64 = chrono::Utc::now().timestamp_millis();
    let ohlcv = Ohlcv::new(
        timestamp_ms,
        dec!(49000.0).into(),
        dec!(51000.0).into(),
        dec!(48500.0).into(),
        dec!(50000.0).into(),
        dec!(1234.5).into(),
    );
    log_success!("Created OHLCV");
    log_field!("Open", ohlcv.open);
    log_field!("High", ohlcv.high);
    log_field!("Low", ohlcv.low);
    log_field!("Close", ohlcv.close);
    log_field!("Volume", ohlcv.volume);

    // Demonstrate timeframe conversion
    log_section!("Timeframe Conversions");
    for timeframe in [Timeframe::M1, Timeframe::H1, Timeframe::D1] {
        log_item!(
            "{} = {} ms = {} seconds",
            timeframe,
            timeframe.as_millis(),
            timeframe.as_seconds()
        );
    }

    // Demonstrate timestamp utilities
    log_section!("Timestamp Utilities");
    let current_timestamp = chrono::Utc::now().timestamp_millis();
    log_field!(
        "Current timestamp (i64)",
        format!("{} ms", current_timestamp)
    );
    log_field!(
        "Timestamp represents",
        chrono::DateTime::from_timestamp_millis(current_timestamp)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default()
    );

    // Demonstrate error handling
    log_section!("Error Handling");
    let err = Error::market_not_found("INVALID/PAIR");
    log_error!("Market not found: {}", err);

    let auth_err = Error::authentication("Invalid API key");
    log_error!("Authentication error: {}", auth_err);

    log_complete!();
    Ok(())
}
