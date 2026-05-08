//! Binance WebSocket data parser module.
//!
//! Parses WebSocket message data into standardized CCXT format structures.
//!
//! ## Module Structure
//!
//! - `ticker` - WebSocket Ticker data parsing (24hrTicker event)
//! - `orderbook` - WebSocket OrderBook data parsing (depthUpdate event)
//! - `trade` - WebSocket Trade data parsing (trade/aggTrade events)
//! - `ohlcv` - WebSocket OHLCV data parsing (kline event)
//! - `balance` - WebSocket Balance data parsing (outboundAccountPosition event)
//! - `order` - WebSocket Order data parsing (executionReport event)
//! - `mark_price` - WebSocket MarkPrice data parsing (markPriceUpdate event)
//! - `position` - WebSocket Position data parsing (ACCOUNT_UPDATE event)
//! - `bidask` - WebSocket BidAsk data parsing (bookTicker event)

mod balance;
mod bidask;
mod mark_price;
mod ohlcv;
mod order;
mod orderbook;
mod position;
mod ticker;
mod trade;

// Re-export all parser functions
pub use balance::{parse_balance, parse_balance_futures};
pub use bidask::parse_bids_asks;
pub use mark_price::parse_mark_price;
pub use ohlcv::parse_ohlcv;
pub use order::{parse_order, parse_order_futures};
pub use orderbook::{parse_orderbook, parse_orderbook_side_ws};
pub use position::parse_positions;
pub use ticker::parse_ticker;
pub use trade::{parse_account_trade, parse_account_trade_futures, parse_trades};

// ============================================================================
// Helper Functions
// ============================================================================

use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

use crate::binance::core::symbol::BinanceSymbolConverter;

/// Convert Binance symbol format to unified format.
///
/// Reads `_ccxt_mt` from the message to determine market type.
/// For swap/future, appends settlement currency suffix (e.g. `BTC/USDT:USDT`).
pub fn to_unified_symbol(msg: &Value) -> String {
    let symbol_str = msg.get("s").and_then(|s| s.as_str()).unwrap_or_default();
    let base = BinanceSymbolConverter::exchange_to_unified_inferred(symbol_str);

    // 读取消息中注入的市场类型标记
    let mt = msg.get("_ccxt_mt").and_then(|v| v.as_str());
    if let Some("swap" | "future") = mt {
        if !base.contains(':') {
            if let Some(slash_pos) = base.rfind('/') {
                let quote = &base[slash_pos + 1..];
                return format!("{}:{}", base, quote);
            }
        }
    }
    base
}

/// Convert Binance raw symbol to unified format with explicit market type.
///
/// For swap/future, appends settlement currency suffix.
pub fn to_unified_symbol_with_mt(symbol_str: &str, market_type: Option<&str>) -> String {
    let base = BinanceSymbolConverter::exchange_to_unified_inferred(symbol_str);
    if let Some("swap" | "future") = market_type {
        if !base.contains(':') {
            if let Some(slash_pos) = base.rfind('/') {
                let quote = &base[slash_pos + 1..];
                return format!("{}:{}", base, quote);
            }
        }
    }
    base
}

/// Parse decimal value from a field.
pub fn parse_decimal(data: &Value, field: &str) -> Option<Decimal> {
    data.get(field)
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
}
