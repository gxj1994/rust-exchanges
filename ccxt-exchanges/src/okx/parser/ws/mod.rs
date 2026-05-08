//! OKX WebSocket data parser module.
//!
//! Parses WebSocket message data into standardized CCXT format structures.
//!
//! ## Module Structure
//!
//! - `ticker` - WebSocket Ticker data parsing
//! - `orderbook` - WebSocket OrderBook data parsing
//! - `trade` - WebSocket Trade data parsing
//! - `ohlcv` - WebSocket OHLCV data parsing
//! - `balance` - WebSocket Balance data parsing
//! - `order` - WebSocket Order data parsing

mod balance;
mod ohlcv;
mod order;
mod orderbook;
mod ticker;
mod trade;

// Re-export all parser functions
pub use balance::parse_balance;
pub use ohlcv::parse_ohlcv;
pub use order::{parse_order, parse_order_from_data, parse_ws_account_trade};
pub use orderbook::parse_orderbook;
pub use ticker::parse_ticker;
pub use trade::parse_trades;

// ============================================================================
// Helper Functions
// ============================================================================

use ccxt_core::error::{Error, Result};
use serde_json::Value;

use crate::okx::core::symbol::OkxSymbolConverter;

/// Convert OKX symbol format to unified format.
pub fn to_unified_symbol(symbol: &str) -> String {
    OkxSymbolConverter::exchange_to_unified_inferred(symbol)
}

/// Extract data array from message.
pub fn extract_data(msg: &Value) -> Result<&Vec<Value>> {
    msg.get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| Error::invalid_request("Missing data in message"))
}

/// Extract first data item from message.
pub fn extract_first_data(msg: &Value) -> Result<&Value> {
    let data = extract_data(msg)?;
    data.first()
        .ok_or_else(|| Error::invalid_request("Empty data array"))
}
