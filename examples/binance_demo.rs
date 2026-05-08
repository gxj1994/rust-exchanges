//! Binance Exchange Complete Demo
//!
//! Comprehensive demonstration of Binance exchange functionality:
//! - Market Data (Public APIs)
//! - Account Management (Authenticated APIs)
//! - Trading Operations (Spot, Margin, Futures)
//! - WebSocket Streams
//!
//! # Usage
//!
//! ```bash
//! # Public APIs only (no credentials required)
//! cargo run --example binance_demo
//!
//! # With authentication for private APIs
//! export BINANCE_API_KEY="your_api_key"
//! export BINANCE_API_SECRET="your_api_secret"
//! cargo run --example binance_demo
//! ```

#![allow(clippy::disallowed_methods)]

use ccxt_core::{
    ExchangeConfig,
    error::Result,
    logging::{LogConfig, init_logging},
    types::{AccountType, TickerParams},
};
use ccxt_exchanges::binance::Binance;
use dotenvy::dotenv;
use rust_decimal::Decimal;
use std::env;

// Include common logging macros
#[path = "common/mod.rs"]
mod common;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logging(&LogConfig::development());

    log_title!("Binance Exchange Demo");

    // Initialize exchange
    let config = create_config()?;
    let exchange = Binance::new(config)?;

    // Run all demo sections
    run_market_data_demo(&exchange).await?;
    run_account_demo(&exchange).await?;
    run_trading_demo(&exchange).await?;
    run_futures_demo(&exchange).await?;

    log_complete!();
    Ok(())
}

fn create_config() -> Result<ExchangeConfig> {
    let mut config = ExchangeConfig::default();
    config.verbose = true;
    config.sandbox = true;

    if let (Ok(api_key), Ok(secret)) = (env::var("BINANCE_API_KEY"), env::var("BINANCE_API_SECRET"))
    {
        config.api_key = Some(ccxt_core::SecretString::new(api_key));
        config.secret = Some(ccxt_core::SecretString::new(secret));
        log_success!("API credentials loaded from environment");
    } else {
        log_warning!("No API credentials found - running in public-only mode");
        log_info!("Set BINANCE_API_KEY and BINANCE_API_SECRET for private API access");
    }

    Ok(config)
}

// =============================================================================
// MARKET DATA (Public APIs)
// =============================================================================
async fn run_market_data_demo(exchange: &Binance) -> Result<()> {
    log_section!("Market Data (Public APIs)");

    // 1. Fetch Markets
    log_subsection!("Fetch Markets");
    match exchange.fetch_markets().await {
        Ok(markets) => {
            log_success!("Fetched {} markets", markets.len());
            use ccxt_core::types::MarketType;
            let spot_count = markets
                .values()
                .filter(|m| m.market_type == MarketType::Spot)
                .count();
            let futures_count = markets
                .values()
                .filter(|m| {
                    m.market_type == MarketType::Swap || m.market_type == MarketType::Futures
                })
                .count();
            log_item!("Spot markets: {}", spot_count);
            log_item!("Futures markets: {}", futures_count);

            if let Some(market) = markets.get("BTC/USDT") {
                log_field!(
                    "Sample",
                    format!("{} ({}/{})", market.symbol, market.base, market.quote)
                );
            }
        }
        Err(e) => log_error!("Failed to fetch markets: {}", e),
    }

    // 2. Fetch Ticker
    log_subsection!("Fetch Ticker");
    match exchange
        .fetch_ticker("BTC/USDT", TickerParams::default())
        .await
    {
        Ok(ticker) => {
            log_success!("Fetched BTC/USDT ticker");
            log_field!("Last Price", format!("{:?}", ticker.last));
            log_field!("24h Change", format!("{:?}%", ticker.percentage));
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
                let spread = best_ask.price - best_bid.price;
                let spread_pct = (spread / best_bid.price) * Decimal::ONE_HUNDRED;
                log_field!("Spread", format!("{} ({:.4}%)", spread, spread_pct));
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

    Ok(())
}

// =============================================================================
// ACCOUNT MANAGEMENT (Authenticated APIs)
// =============================================================================
async fn run_account_demo(exchange: &Binance) -> Result<()> {
    log_section!("Account Management (Authenticated)");

    // Check if we have credentials
    if env::var("BINANCE_API_KEY").is_err() {
        log_skipped!("Account demo - no API credentials");
        return Ok(());
    }

    // 1. Spot Balance
    log_subsection!("Spot Account Balance");
    match exchange.fetch_balance(Some(AccountType::Spot)).await {
        Ok(balance) => {
            log_success!("Fetched spot balance");
            let non_zero: Vec<_> = balance
                .balances
                .iter()
                .filter(|(_, entry)| entry.total > Decimal::ZERO)
                .take(5)
                .collect();

            if non_zero.is_empty() {
                log_info!("No non-zero balances found");
            } else {
                for (currency, entry) in non_zero {
                    log_item!(
                        "{}: Total={}, Free={}, Used={}",
                        currency,
                        entry.total,
                        entry.free,
                        entry.used
                    );
                }
            }
        }
        Err(e) => log_error!("Failed to fetch spot balance: {}", e),
    }

    // 2. Futures Balance
    log_subsection!("Futures Account Balance");
    match exchange.fetch_balance(Some(AccountType::Futures)).await {
        Ok(balance) => {
            log_success!("Fetched futures balance");
            let non_zero: Vec<_> = balance
                .balances
                .iter()
                .filter(|(_, entry)| entry.total > Decimal::ZERO)
                .take(3)
                .collect();

            for (currency, entry) in non_zero {
                log_item!("{}: {}", currency, entry.total);
            }
        }
        Err(e) => log_error!("Failed to fetch futures balance: {}", e),
    }

    // 3. Funding Balance
    log_subsection!("Funding Account Balance");
    match exchange.fetch_balance(Some(AccountType::Funding)).await {
        Ok(balance) => {
            log_success!("Fetched funding balance");
            let count = balance.balances.len();
            log_item!("Total currencies: {}", count);
        }
        Err(e) => log_error!("Failed to fetch funding balance: {}", e),
    }

    Ok(())
}

// =============================================================================
// TRADING OPERATIONS (Authenticated APIs)
// =============================================================================
async fn run_trading_demo(exchange: &Binance) -> Result<()> {
    log_section!("Trading Operations (Authenticated)");

    if env::var("BINANCE_API_KEY").is_err() {
        log_skipped!("Trading demo - no API credentials");
        return Ok(());
    }

    // Load markets for symbol resolution
    if let Err(e) = exchange.load_markets(false).await {
        log_error!("Failed to load markets: {}", e);
        return Ok(());
    }

    // 1. Fetch Open Orders
    log_subsection!("Open Orders");
    match exchange.fetch_open_orders(Some("BTC/USDT")).await {
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

    // 2. Fetch Order History
    log_subsection!("Order History");
    match exchange
        .fetch_history_orders(Some("BTC/USDT"), None, Some(5))
        .await
    {
        Ok(orders) => {
            if orders.is_empty() {
                log_info!("No order history found");
            } else {
                log_success!("Found {} historical orders", orders.len());
            }
        }
        Err(e) => log_error!("Failed to fetch order history: {}", e),
    }

    // 3. My Trades
    log_subsection!("My Trades");
    match exchange
        .fetch_account_trades("BTC/USDT", None, Some(5))
        .await
    {
        Ok(trades) => {
            if trades.is_empty() {
                log_info!("No trades found");
            } else {
                log_success!("Found {} trades", trades.len());
                for trade in trades.iter().take(3) {
                    log_item!(
                        "{:?} {} @ {} (fee: {:?})",
                        trade.side,
                        trade.amount,
                        trade.price,
                        trade.fee.as_ref().map(|f| &f.cost)
                    );
                }
            }
        }
        Err(e) => log_error!("Failed to fetch my trades: {}", e),
    }

    Ok(())
}

// =============================================================================
// FUTURES OPERATIONS
// =============================================================================
async fn run_futures_demo(_exchange: &Binance) -> Result<()> {
    log_section!("Futures Operations");

    // Create futures exchange instance
    let futures_config = ExchangeConfig {
        sandbox: true,
        ..Default::default()
    };
    let futures = Binance::new_swap(futures_config)?;

    // 1. Fetch Futures Markets
    log_subsection!("Futures Markets");
    match futures.fetch_markets().await {
        Ok(markets) => {
            let perpetuals: Vec<_> = markets
                .values()
                .filter(|m| m.symbol.contains("PERP"))
                .take(5)
                .collect();
            log_success!("Found {} perpetual futures", perpetuals.len());
            for market in perpetuals {
                log_item!("{}", market.symbol);
            }
        }
        Err(e) => log_error!("Failed to fetch futures markets: {}", e),
    }

    // 2. Fetch Futures Ticker
    log_subsection!("Futures Ticker");
    match futures
        .fetch_ticker("BTC/USDT:USDT", TickerParams::default())
        .await
    {
        Ok(ticker) => {
            log_success!("Fetched BTC/USDT:USDT ticker");
            log_field!("Last Price", format!("{:?}", ticker.last));
            log_field!("Mark Price", format!("{:?}", ticker.mark_price));
            log_field!("Funding Rate", format!("{:?}", ticker.funding_rate));
        }
        Err(e) => log_error!("Failed to fetch futures ticker: {}", e),
    }

    // 3. Fetch Positions (requires auth)
    if env::var("BINANCE_API_KEY").is_ok() {
        log_subsection!("Open Positions");
        match futures.fetch_positions(None, None).await {
            Ok(positions) => {
                let open: Vec<_> = positions
                    .iter()
                    .filter(|p| p.contracts.unwrap_or(0.0) != 0.0)
                    .collect();
                if open.is_empty() {
                    log_info!("No open positions");
                } else {
                    log_success!("Found {} open positions", open.len());
                    for pos in open.iter().take(3) {
                        log_item!(
                            "{}: {} contracts, PnL: {:?}",
                            pos.symbol,
                            pos.contracts.unwrap_or(0.0),
                            pos.unrealized_pnl
                        );
                    }
                }
            }
            Err(e) => log_error!("Failed to fetch positions: {}", e),
        }
    }

    Ok(())
}
