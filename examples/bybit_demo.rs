//! Bybit Exchange Complete Demo
//!
//! Comprehensive demonstration of Bybit exchange functionality:
//! - Market Data (Public APIs)
//! - Account Management (Authenticated APIs)
//! - Trading Operations (Spot, Perpetual, Futures)
//! - WebSocket Streams
//!
//! # Usage
//!
//! ```bash
//! # Public APIs only (no credentials required)
//! cargo run --example bybit_demo
//!
//! # With authentication for private APIs
//! export BYBIT_API_KEY="your_api_key"
//! export BYBIT_API_SECRET="your_api_secret"
//! cargo run --example bybit_demo
//! ```

#![allow(clippy::disallowed_methods)]

use ccxt_core::{
    error::Result,
    logging::{LogConfig, init_logging},
};
use ccxt_exchanges::bybit::BybitBuilder;
use dotenvy::dotenv;
use std::env;

// Include common logging macros
#[path = "common/mod.rs"]
mod common;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logging(&LogConfig::development());

    log_title!("Bybit Exchange Demo");

    // Initialize exchange
    let exchange = create_exchange().await?;

    // Run all demo sections
    run_market_data_demo(&exchange).await?;
    run_account_demo(&exchange).await?;
    run_trading_demo(&exchange).await?;

    log_complete!();
    Ok(())
}

async fn create_exchange() -> Result<ccxt_exchanges::bybit::Bybit> {
    let mut builder = BybitBuilder::new().testnet(true);

    if let (Ok(api_key), Ok(secret)) = (env::var("BYBIT_API_KEY"), env::var("BYBIT_API_SECRET")) {
        builder = builder.api_key(&api_key).secret(&secret);
        log_success!("API credentials loaded from environment");
    } else {
        log_warning!("No API credentials found - running in public-only mode");
        log_info!("Set BYBIT_API_KEY and BYBIT_API_SECRET for private API access");
    }

    builder.build()
}

// =============================================================================
// MARKET DATA (Public APIs)
// =============================================================================
async fn run_market_data_demo(exchange: &ccxt_exchanges::bybit::Bybit) -> Result<()> {
    log_section!("Market Data (Public APIs)");

    // Display Exchange Info
    log_subsection!("Exchange Information");
    log_field!("Name", exchange.name());
    log_field!("ID", exchange.id());
    log_field!("Version", exchange.version());
    log_field!("Testnet", exchange.options().testnet);
    log_field!("Account Type", exchange.options().account_type);
    log_field!(
        "Rate Limit",
        format!("{} req/s", exchange.requests_per_second())
    );

    // 1. Fetch Markets
    log_subsection!("Fetch Markets");
    match exchange.fetch_markets().await {
        Ok(markets) => {
            log_success!("Fetched {} markets", markets.len());
            let sample: Vec<_> = markets.values().take(5).collect();
            log_info!("Sample markets:");
            for market in sample {
                log_item!(
                    "{} - Base: {}, Quote: {}, Active: {}",
                    market.symbol,
                    market.base,
                    market.quote,
                    market.active
                );
            }
        }
        Err(e) => log_error!("Failed to fetch markets: {}", e),
    }

    // Load markets for subsequent calls
    if let Err(e) = exchange.load_markets(false).await {
        log_error!("Failed to load markets: {}", e);
        return Ok(());
    }

    // 2. Fetch Ticker
    log_subsection!("Fetch Ticker");
    match exchange.fetch_ticker("BTC/USDT").await {
        Ok(ticker) => {
            log_success!("Fetched BTC/USDT ticker");
            log_field!("Symbol", ticker.symbol);
            if let Some(last) = ticker.last {
                log_field!("Last Price", last);
            }
            if let Some(high) = ticker.high {
                log_field!("24h High", high);
            }
            if let Some(low) = ticker.low {
                log_field!("24h Low", low);
            }
            log_field!("Timestamp", ticker.timestamp);
        }
        Err(e) => log_error!("Failed to fetch ticker: {}", e),
    }

    // 3. Fetch Order Book
    log_subsection!("Fetch Order Book");
    match exchange.fetch_order_book("BTC/USDT", Some(5)).await {
        Ok(book) => {
            log_success!("Fetched order book");
            log_field!("Bids", book.bids.len());
            log_field!("Asks", book.asks.len());
            if let Some(best_bid) = book.bids.first() {
                log_field!(
                    "Best Bid",
                    format!("{} @ {}", best_bid.amount, best_bid.price)
                );
            }
            if let Some(best_ask) = book.asks.first() {
                log_field!(
                    "Best Ask",
                    format!("{} @ {}", best_ask.amount, best_ask.price)
                );
            }
        }
        Err(e) => log_error!("Failed to fetch order book: {}", e),
    }

    // 4. Fetch Recent Trades
    log_subsection!("Fetch Recent Trades");
    match exchange.fetch_market_trades("BTC/USDT", Some(5)).await {
        Ok(trades) => {
            log_success!("Fetched {} recent trades", trades.len());
            for trade in trades.iter().take(3) {
                log_item!(
                    "{:?} {} @ {} ({})",
                    trade.side,
                    trade.amount,
                    trade.price,
                    trade.timestamp
                );
            }
        }
        Err(e) => log_error!("Failed to fetch trades: {}", e),
    }

    // 5. Display Supported Timeframes
    log_subsection!("Supported Timeframes");
    let timeframes = exchange.timeframes();
    let mut tf_list: Vec<_> = timeframes.keys().collect();
    tf_list.sort();
    for tf in tf_list.iter().take(6) {
        if let Some(value) = timeframes.get(*tf) {
            log_item!("{} -> {}", tf, value);
        }
    }
    if timeframes.len() > 6 {
        log_info!("... and {} more", timeframes.len() - 6);
    }

    Ok(())
}

// =============================================================================
// ACCOUNT MANAGEMENT (Authenticated APIs)
// =============================================================================
async fn run_account_demo(exchange: &ccxt_exchanges::bybit::Bybit) -> Result<()> {
    log_section!("Account Management (Authenticated)");

    // Check if we have credentials
    if env::var("BYBIT_API_KEY").is_err() {
        log_skipped!("Account demo - no API credentials");
        return Ok(());
    }

    // 1. Fetch Account Balance
    log_subsection!("Account Balance");
    match exchange.fetch_balance().await {
        Ok(balance) => {
            log_success!("Fetched account balance");
            if let Some(usdt) = balance.get("USDT") {
                log_item!("USDT Balance:");
                log_field!("Total", usdt.total);
                log_field!("Free", usdt.free);
                log_field!("Used", usdt.used);
            } else {
                log_info!("No USDT balance found");
            }
        }
        Err(e) => log_error!("Failed to fetch balance: {}", e),
    }

    Ok(())
}

// =============================================================================
// TRADING OPERATIONS (Authenticated APIs)
// =============================================================================
async fn run_trading_demo(exchange: &ccxt_exchanges::bybit::Bybit) -> Result<()> {
    log_section!("Trading Operations (Authenticated)");

    if env::var("BYBIT_API_KEY").is_err() {
        log_skipped!("Trading demo - no API credentials");
        return Ok(());
    }

    // 1. Fetch Open Orders
    log_subsection!("Open Orders");
    match exchange
        .fetch_open_orders(Some("BTC/USDT"), None, None)
        .await
    {
        Ok(orders) => {
            if orders.is_empty() {
                log_info!("No open orders found");
            } else {
                log_success!("Found {} open orders", orders.len());
                for order in orders.iter().take(3) {
                    log_item!(
                        "{} - {:?} {:?} {} @ {:?}",
                        order.id,
                        order.side,
                        order.order_type,
                        order.amount,
                        order.price
                    );
                }
            }
        }
        Err(e) => log_error!("Failed to fetch open orders: {}", e),
    }

    Ok(())
}
