//! HyperLiquid WebSocket data parser module.
//!
//! Parses WebSocket message data into standardized CCXT format structures.
//!
//! ## Module Structure
//!
//! - `ticker` - WebSocket Ticker data parsing (allMids channel)
//! - `orderbook` - WebSocket OrderBook data parsing (l2Book channel)
//! - `trade` - WebSocket Trade data parsing (trades/userFills channels)
//! - `ohlcv` - WebSocket OHLCV data parsing (candle channel)
//! - `balance` - WebSocket Balance data parsing (userEvents channel)
//! - `order` - WebSocket Order data parsing (orderUpdates channel)

mod balance;
mod bidask;
mod ohlcv;
mod order;
mod orderbook;
mod ticker;
mod trade;

// Re-export all parser functions
pub use balance::parse_balance;
pub use bidask::parse_bids_asks;
pub use ohlcv::parse_ohlcv;
pub use order::{parse_order, parse_order_from_data, parse_order_update};
pub use orderbook::parse_orderbook;
pub use ticker::{parse_all_mids, parse_all_mids_map};
pub use trade::{parse_trades, parse_user_fills};

// ============================================================================
// Helper Functions
// ============================================================================

use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;

/// Helper to parse decimal from a JSON value directly.
///
/// Supports both number and string representations.
pub fn parse_decimal_from_value(v: &Value) -> Option<Decimal> {
    if let Some(num) = v.as_f64() {
        Decimal::from_f64(num)
    } else if let Some(s) = v.as_str() {
        Decimal::from_str(s).ok()
    } else {
        None
    }
}
