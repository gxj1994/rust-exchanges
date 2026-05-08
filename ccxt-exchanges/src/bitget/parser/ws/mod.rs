//! Bitget WebSocket data parser module.
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

use crate::bitget::core::symbol::BitgetSymbolConverter;

/// Convert Bitget symbol format to unified format.
///
/// Reads `_ccxt_mt` from the message to determine market type.
/// For swap/future, appends settlement currency suffix (e.g. `BTC/USDT:USDT`).
pub fn to_unified_symbol(symbol: &str, msg: Option<&Value>) -> String {
    // 优先用 Bitget 消息自带的 `arg.instType` 判断市场类型
    // （V3 UTA 消息中 instType 为 "SPOT" / "USDT-FUTURES" / "COIN-FUTURES"）
    let product_type = msg
        .and_then(|m| m.get("arg"))
        .and_then(|a| a.get("instType"))
        .and_then(|t| t.as_str())
        .filter(|t| !t.is_empty());

    // 有明确 instType 时走精确转换，否则回退到 inferred
    match product_type {
        Some("USDT-FUTURES" | "COIN-FUTURES" | "usdt-futures" | "coin-futures") => {
            BitgetSymbolConverter::exchange_to_unified_hint(symbol, product_type.unwrap())
        }
        _ => {
            let base = BitgetSymbolConverter::exchange_to_unified_inferred(symbol);

            // 回退：读取注入的 `_ccxt_mt` 标记
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
    }
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
