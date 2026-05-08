//! WebSocket MarkPrice data parser for Binance.
//!
//! Parses markPriceUpdate events.

use ccxt_core::{error::Result, types::MarkPrice};
use serde_json::Value;

use super::{parse_decimal, to_unified_symbol};

/// Parse mark price from WebSocket markPriceUpdate message.
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`MarkPrice`] structure.
pub fn parse_mark_price(msg: &Value) -> Result<MarkPrice> {
    let symbol = to_unified_symbol(msg);

    let timestamp = msg.get("E").and_then(|t| t.as_i64()).unwrap_or(0);

    let mark_price = parse_decimal(msg, "p").unwrap_or_default();
    let index_price = parse_decimal(msg, "i");
    let funding_rate = parse_decimal(msg, "r");
    let next_funding_time = msg.get("T").and_then(|t| t.as_i64());

    Ok(MarkPrice::new(
        symbol,
        mark_price,
        index_price,
        None, // estimated_settle_price
        funding_rate,
        next_funding_time,
        None, // interest_rate
        timestamp,
    ))
}
