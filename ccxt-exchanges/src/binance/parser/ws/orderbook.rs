//! WebSocket OrderBook data parser for Binance.
//!
//! Parses depthUpdate events.

use ccxt_core::{
    error::Result,
    types::{
        OrderBook, OrderBookEntry, Symbol,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

use super::to_unified_symbol;

/// Parse orderbook from WebSocket depthUpdate message.
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure.
pub fn parse_orderbook(msg: &Value) -> Result<OrderBook> {
    let symbol = Symbol::new_unchecked(to_unified_symbol(msg));

    let timestamp = msg.get("E").and_then(|t| t.as_i64()).unwrap_or(0);

    // Get nonce (lastUpdateId)
    let nonce = msg
        .get("u")
        .and_then(|u| u.as_i64())
        .or_else(|| msg.get("lastUpdateId").and_then(|u| u.as_i64()));

    // 支持两种格式：
    // 1. 快照流：使用 "bids" 和 "asks" 字段
    // 2. 增量流：使用 "b" 和 "a" 字段
    let bids = msg
        .get("bids")
        .or_else(|| msg.get("b"))
        .and_then(|b| b.as_array())
        .map(|arr| parse_orderbook_side_ws(&Value::Array(arr.clone())))
        .unwrap_or_default();

    let asks = msg
        .get("asks")
        .or_else(|| msg.get("a"))
        .and_then(|a| a.as_array())
        .map(|arr| parse_orderbook_side_ws(&Value::Array(arr.clone())))
        .unwrap_or_default();

    let mut ob = OrderBook::new(symbol, timestamp);
    ob.bids = bids;
    ob.asks = asks;
    ob.nonce = nonce;
    ob.is_synced = true; // Snapshot stream is synced by default
    Ok(ob)
}

/// Parse WebSocket OrderBook side (bids or asks).
pub fn parse_orderbook_side_ws(data: &Value) -> Vec<OrderBookEntry> {
    let array = match data.as_array() {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    let mut result = Vec::with_capacity(array.len());

    for item in array {
        if let Some(arr) = item.as_array() {
            if arr.len() >= 2 {
                let price = arr[0].as_str().and_then(|s| Decimal::from_str(s).ok());

                let amount = arr[1].as_str().and_then(|s| Decimal::from_str(s).ok());

                if let (Some(price), Some(amount)) = (price, amount) {
                    result.push(OrderBookEntry {
                        price: Price::new(price),
                        amount: Amount::new(amount),
                    });
                }
            }
        }
    }

    result
}
