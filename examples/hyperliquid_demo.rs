//! HyperLiquid Exchange Complete Demo
//!
//! Comprehensive demonstration of HyperLiquid exchange functionality:
//! - Market Data (Public APIs)
//! - Account Management (Authenticated APIs)
//! - Trading Operations (Perpetual Futures)
//!
//! # Usage
//!
//! ```bash
//! # Public APIs only (no credentials required)
//! cargo run --example hyperliquid_demo
//!
//! # With authentication for private APIs
//! export HYPERLIQUID_PRIVATE_KEY="your_private_key"
//! cargo run --example hyperliquid_demo
//! ```

#![allow(clippy::disallowed_methods)]

use ccxt_core::{
    error::Result,
    exchange::Exchange,
    logging::{LogConfig, init_logging},
};
use ccxt_exchanges::hyperliquid::HyperLiquidBuilder;
use dotenvy::dotenv;
use std::env;

// Include common logging macros
#[path = "common/mod.rs"]
mod common;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logging(&LogConfig::development());

    log_title!("HyperLiquid Exchange Demo");

    // Initialize exchange
    let exchange = create_exchange().await?;

    // Run all demo sections
    run_market_data_demo(&exchange).await?;
    run_account_demo(&exchange).await?;
    run_trading_demo(&exchange).await?;

    log_complete!();
    Ok(())
}

async fn create_exchange() -> Result<ccxt_exchanges::hyperliquid::HyperLiquid> {
    let mut builder = HyperLiquidBuilder::new().testnet(true);

    if let Ok(private_key) = env::var("HYPERLIQUID_PRIVATE_KEY") {
        builder = builder.private_key(&private_key);
        log_success!("Private key loaded from environment");
    } else {
        log_warning!("No private key found - running in public-only mode");
        log_info!("Set HYPERLIQUID_PRIVATE_KEY for private API access");
    }

    builder.build()
}

// =============================================================================
// MARKET DATA (Public APIs)
// =============================================================================
async fn run_market_data_demo(exchange: &ccxt_exchanges::hyperliquid::HyperLiquid) -> Result<()> {
    log_section!("Market Data (Public APIs)");

    // Display Exchange Info
    log_subsection!("Exchange Information");
    log_field!("Name", exchange.name());
    log_field!("ID", exchange.id());
    log_field!("Testnet", exchange.options().testnet);

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
    match exchange.fetch_ticker("BTC/USDC:USDC").await {
        Ok(ticker) => {
            log_success!("Fetched BTC/USDC:USDC ticker");
            log_field!("Symbol", ticker.symbol);
            if let Some(last) = ticker.last {
                log_field!("Last Price", last);
            }
            log_field!("Timestamp", ticker.timestamp);
        }
        Err(e) => log_error!("Failed to fetch ticker: {}", e),
    }

    // 3. Fetch Order Book
    log_subsection!("Fetch Order Book");
    match exchange.fetch_order_book("BTC/USDC:USDC", Some(5)).await {
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
    match exchange.fetch_market_trades("BTC/USDC:USDC", Some(5)).await {
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

    // 5. Display Capabilities
    log_subsection!("Exchange Capabilities");
    let caps = exchange.capabilities();
    log_item!("fetch_markets: {}", caps.fetch_markets());
    log_item!("fetch_ticker: {}", caps.fetch_ticker());
    log_item!("fetch_order_book: {}", caps.fetch_order_book());
    log_item!("fetch_trades: {}", caps.fetch_trades());
    log_item!("fetch_ohlcv: {}", caps.fetch_ohlcv());
    log_item!("create_order: {}", caps.create_order());
    log_item!("cancel_order: {}", caps.cancel_order());
    log_item!("fetch_balance: {}", caps.fetch_balance());
    log_item!("fetch_positions: {}", caps.fetch_positions());
    log_item!("set_leverage: {}", caps.set_leverage());

    Ok(())
}

// =============================================================================
// ACCOUNT MANAGEMENT (Authenticated APIs)
// =============================================================================
async fn run_account_demo(exchange: &ccxt_exchanges::hyperliquid::HyperLiquid) -> Result<()> {
    log_section!("Account Management (Authenticated)");

    // Check if we have credentials
    if env::var("HYPERLIQUID_PRIVATE_KEY").is_err() {
        log_skipped!("Account demo - no private key");
        return Ok(());
    }

    // Display wallet address
    if let Some(address) = exchange.wallet_address() {
        log_subsection!("Wallet Information");
        log_field!("Address", address);
    }

    // 1. Fetch Account Balance
    log_subsection!("Account Balance");
    match exchange.fetch_balance().await {
        Ok(balance) => {
            log_success!("Fetched account balance");
            if let Some(usdc) = balance.get("USDC") {
                log_item!("USDC Balance:");
                log_field!("Total", usdc.total);
                log_field!("Free", usdc.free);
                log_field!("Used", usdc.used);
            } else {
                log_info!("No USDC balance found");
            }
        }
        Err(e) => log_error!("Failed to fetch balance: {}", e),
    }

    // 2. Fetch Positions
    log_subsection!("Open Positions");
    match exchange.fetch_positions(None).await {
        Ok(positions) => {
            if positions.is_empty() {
                log_info!("No open positions");
            } else {
                log_success!("Found {} positions", positions.len());
                for pos in positions.iter().take(3) {
                    log_item!(
                        "{} - Size: {:?}, Entry: {:?}, PnL: {:?}",
                        pos.symbol,
                        pos.contracts,
                        pos.entry_price,
                        pos.unrealized_pnl
                    );
                }
            }
        }
        Err(e) => log_error!("Failed to fetch positions: {}", e),
    }

    Ok(())
}

// =============================================================================
// TRADING OPERATIONS (Authenticated APIs)
// =============================================================================
async fn run_trading_demo(_exchange: &ccxt_exchanges::hyperliquid::HyperLiquid) -> Result<()> {
    log_section!("Trading Operations (Authenticated)");

    if env::var("HYPERLIQUID_PRIVATE_KEY").is_err() {
        log_skipped!("Trading demo - no private key");
        return Ok(());
    }

    log_info!("Trading operations would be executed here with valid credentials");
    log_info!("Examples: create_order, cancel_order, set_leverage, etc.");

    Ok(())
}
