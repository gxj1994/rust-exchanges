//! Bybit WebSocket data parser module.
//!
//! Parses WebSocket message data into standardized CCXT format structures.
//!
//! ## Module Structure
//!
//! - `bidask` - WebSocket BidsAsks (orderbook.1) data parsing
//! - `ticker` - WebSocket Ticker data parsing
//! - `orderbook` - WebSocket OrderBook data parsing
//! - `trade` - WebSocket Trade data parsing
//! - `ohlcv` - WebSocket OHLCV data parsing
//! - `balance` - WebSocket Balance data parsing
//! - `order` - WebSocket Order data parsing

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
pub use order::{parse_order, parse_order_from_data};
pub use orderbook::parse_orderbook;
pub use ticker::parse_ticker;
pub use trade::{parse_execution, parse_trades};

// ============================================================================
// Helper Functions
// ============================================================================

use ccxt_core::error::{Error, Result};
use serde_json::Value;

use crate::bybit::core::symbol::BybitSymbolConverter;

/// Convert Bybit symbol format to unified format.
///
/// Reads `_ccxt_mt` from the message to determine market type.
/// For swap/future, appends settlement currency suffix (e.g. `BTC/USDT:USDT`).
pub fn to_unified_symbol(symbol: &str, msg: Option<&Value>) -> String {
    let base = BybitSymbolConverter::exchange_to_unified_inferred(symbol);

    if let Some(msg) = msg {
        let mt = msg.get("_ccxt_mt").and_then(|v| v.as_str());
        if let Some("swap" | "future") = mt {
            if !base.contains(':') {
                if let Some(slash_pos) = base.rfind('/') {
                    let quote = &base[slash_pos + 1..];
                    return format!("{}:{}", base, quote);
                }
            }
        }
    }
    base
}

/// Extract data from message.
pub fn extract_data(msg: &Value) -> Result<&Value> {
    msg.get("data")
        .ok_or_else(|| Error::invalid_request("Missing data in message"))
}
