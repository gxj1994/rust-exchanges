//! OKX Exchange Complete Demo
//!
//! Comprehensive demonstration of OKX exchange functionality:
//! - Market Data (Public APIs)
//! - Account Management (Authenticated APIs)
//! - Trading Operations (Spot, Perpetual Swap)
//! - WebSocket Streams
//!
//! # Usage
//!
//! ```bash
//! # Public APIs only (no credentials required)
//! cargo run --example okx_demo
//!
//! # With authentication for private APIs
//! export OKX_API_KEY="your_api_key"
//! export OKX_API_SECRET="your_api_secret"
//! export OKX_PASSPHRASE="your_passphrase"
//! cargo run --example okx_demo
//! ```

#![allow(clippy::disallowed_methods)]

use ccxt_core::{
    DefaultType,
    error::Result,
    exchange::Exchange,
    logging::{LogConfig, init_logging},
};
use ccxt_exchanges::okx::OkxBuilder;
use dotenvy::dotenv;
use std::env;

// Include common logging macros
#[path = "common/mod.rs"]
mod common;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logging(&LogConfig::development());

    log_title!("OKX Exchange Demo");

    // Initialize exchange
    let exchange = create_exchange().await?;

    // Run all demo sections
    run_market_data_demo(&exchange).await?;
    run_account_demo(&exchange).await?;
    run_trading_demo(&exchange).await?;
    run_futures_demo().await?;

    log_complete!();
    Ok(())
}

async fn create_exchange() -> Result<ccxt_exchanges::okx::Okx> {
    let mut builder = OkxBuilder::new().sandbox(true);

    if let (Ok(api_key), Ok(secret), Ok(passphrase)) = (
        env::var("OKX_API_KEY"),
        env::var("OKX_API_SECRET"),
        env::var("OKX_PASSPHRASE"),
    ) {
        builder = builder
            .api_key(&api_key)
            .secret(&secret)
            .passphrase(&passphrase);
        log_success!("API credentials loaded from environment");
    } else {
        log_warning!("No API credentials found - running in public-only mode");
        log_info!("Set OKX_API_KEY, OKX_API_SECRET, and OKX_PASSPHRASE for private API access");
    }

    builder.build()
}

// =============================================================================
// MARKET DATA (Public APIs)
// =============================================================================
async fn run_market_data_demo(exchange: &ccxt_exchanges::okx::Okx) -> Result<()> {
    log_section!("Market Data (Public APIs)");

    // 1. Fetch Markets
    log_subsection!("Fetch Markets");
    match exchange.fetch_markets().await {
        Ok(markets) => {
            log_success!("Fetched {} markets", markets.len());
            let sample: Vec<_> = markets.values().take(5).collect();
            log_info!("Sample markets:");
            for market in sample {
                log_item!("{} ({}/{})", market.symbol, market.base, market.quote);
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
            log_field!("Last Price", format!("{:?}", ticker.last));
            log_field!("24h High", format!("{:?}", ticker.high));
            log_field!("24h Low", format!("{:?}", ticker.low));
            log_field!("24h Volume", format!("{:?}", ticker.base_volume));
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
            if let (Some(best_bid), Some(best_ask)) = (book.bids.first(), book.asks.first()) {
                log_field!(
                    "Best Bid",
                    format!("{} @ {}", best_bid.amount, best_bid.price)
                );
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
            for (i, trade) in trades.iter().take(3).enumerate() {
                log_item!(
                    "{}: {:?} {} @ {}",
                    i + 1,
                    trade.side,
                    trade.amount,
                    trade.price
                );
            }
        }
        Err(e) => log_error!("Failed to fetch trades: {}", e),
    }

    // 5. Display Exchange Info
    log_subsection!("Exchange Information");
    log_field!("ID", exchange.id());
    log_field!("Name", exchange.name());
    log_field!("Version", exchange.version());
    log_field!("Testnet", exchange.options().testnet);

    // 6. Display Capabilities
    log_subsection!("Exchange Capabilities");
    let caps = exchange.capabilities();
    log_item!("fetch_markets: {}", caps.fetch_markets());
    log_item!("fetch_ticker: {}", caps.fetch_ticker());
    log_item!("fetch_order_book: {}", caps.fetch_order_book());
    log_item!("fetch_trades: {}", caps.fetch_trades());
    log_item!("create_order: {}", caps.create_order());
    log_item!("fetch_balance: {}", caps.fetch_balance());

    Ok(())
}

// =============================================================================
// ACCOUNT MANAGEMENT (Authenticated APIs)
// =============================================================================
async fn run_account_demo(exchange: &ccxt_exchanges::okx::Okx) -> Result<()> {
    log_section!("Account Management (Authenticated)");

    // Check if we have credentials
    if env::var("OKX_API_KEY").is_err() {
        log_skipped!("Account demo - no API credentials");
        return Ok(());
    }

    // 1. Fetch Account Balance
    log_subsection!("Account Balance");
    match exchange.fetch_balance().await {
        Ok(balance) => {
            log_success!("Fetched account balance");
            let mut found_any = false;
            for (currency, entry) in balance.balances.iter() {
                if entry.total > rust_decimal::Decimal::ZERO {
                    log_item!(
                        "{}: Total={}, Free={}, Used={}",
                        currency,
                        entry.total,
                        entry.free,
                        entry.used
                    );
                    found_any = true;
                }
            }
            if !found_any {
                log_info!("No non-zero balances found");
            }
        }
        Err(e) => log_error!("Failed to fetch balance: {}", e),
    }

    // 2. Fetch My Trades
    log_subsection!("My Trades (BTC/USDT)");
    match exchange
        .fetch_account_trades("BTC/USDT", None, Some(5))
        .await
    {
        Ok(trades) => {
            if trades.is_empty() {
                log_info!("No recent trades found");
            } else {
                log_success!("Found {} recent trades", trades.len());
                for (i, trade) in trades.iter().enumerate() {
                    log_item!(
                        "{}: {:?} {} @ {} ({})",
                        i + 1,
                        trade.side,
                        trade.amount,
                        trade.price,
                        trade.datetime.as_deref().unwrap_or("N/A")
                    );
                }
            }
        }
        Err(e) => log_error!("Failed to fetch my trades: {}", e),
    }

    Ok(())
}

// =============================================================================
// TRADING OPERATIONS (Authenticated APIs)
// =============================================================================
async fn run_trading_demo(exchange: &ccxt_exchanges::okx::Okx) -> Result<()> {
    log_section!("Trading Operations (Authenticated)");

    if env::var("OKX_API_KEY").is_err() {
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
                        "{}: {:?} {:?} {} @ {:?}",
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

// =============================================================================
// FUTURES OPERATIONS
// =============================================================================
async fn run_futures_demo() -> Result<()> {
    log_section!("Futures Operations (Perpetual Swap)");

    // Create futures exchange instance
    let futures = OkxBuilder::new().default_type(DefaultType::Swap).build()?;

    // 1. Fetch Swap Markets
    log_subsection!("Swap Markets");
    match futures.fetch_markets().await {
        Ok(markets) => {
            let swaps: Vec<_> = markets
                .values()
                .filter(|m| m.quote == "USDT")
                .take(5)
                .collect();
            log_success!("Found {} USDT-margined swaps", swaps.len());
            for market in swaps {
                log_item!("{} (Linear: {:?})", market.symbol, market.linear);
            }
        }
        Err(e) => log_error!("Failed to fetch swap markets: {}", e),
    }

    // Load markets for subsequent calls
    if let Err(e) = futures.load_markets(false).await {
        log_error!("Failed to load markets: {}", e);
        return Ok(());
    }

    // 2. Fetch Swap Ticker
    log_subsection!("Swap Ticker");
    match futures.fetch_ticker("BTC/USDT:USDT").await {
        Ok(ticker) => {
            log_success!("Fetched BTC/USDT:USDT ticker");
            log_field!("Last Price", format!("{:?}", ticker.last));
            log_field!("Mark Price", format!("{:?}", ticker.mark_price));
            log_field!("Index Price", format!("{:?}", ticker.index_price));
            log_field!("Open Interest", format!("{:?}", ticker.open_interest));
        }
        Err(e) => {
            log_error!("Failed to fetch swap ticker: {}", e);
            // Try fallback
            log_info!("Trying BTC/USDT...");
            match futures.fetch_ticker("BTC/USDT").await {
                Ok(ticker) => log_success!("Fetched BTC/USDT: {:?}", ticker.last),
                Err(e2) => log_error!("Fallback also failed: {}", e2),
            }
        }
    }

    // 3. Fetch Swap Order Book
    log_subsection!("Swap Order Book");
    match futures.fetch_order_book("BTC/USDT:USDT", Some(5)).await {
        Ok(book) => {
            log_success!("Fetched swap order book");
            if let (Some(best_bid), Some(best_ask)) = (book.bids.first(), book.asks.first()) {
                log_field!(
                    "Best Bid",
                    format!("{} @ {}", best_bid.amount, best_bid.price)
                );
                log_field!(
                    "Best Ask",
                    format!("{} @ {}", best_ask.amount, best_ask.price)
                );
            }
        }
        Err(e) => log_error!("Failed to fetch swap order book: {}", e),
    }

    Ok(())
}
