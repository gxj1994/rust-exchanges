//! WebSocket Ticker data parser for Binance.
//!
//! Parses 24hrTicker and 24hrMiniTicker events.

use ccxt_core::{
    error::Result,
    types::{
        Symbol, Ticker,
        financial::{Amount, Price},
    },
};
use serde_json::Value;

use super::{parse_decimal, to_unified_symbol};

/// Parse ticker from WebSocket 24hrTicker message.
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
pub fn parse_ticker(msg: &Value) -> Result<Ticker> {
    let symbol = Symbol::new_unchecked(to_unified_symbol(msg));

    let timestamp = msg.get("E").and_then(|t| t.as_i64()).unwrap_or(0);

    let last = parse_decimal(msg, "c").map(Price::new);
    let open = parse_decimal(msg, "o").map(Price::new);
    let high = parse_decimal(msg, "h").map(Price::new);
    let low = parse_decimal(msg, "l").map(Price::new);
    let bid = parse_decimal(msg, "b").map(Price::new);
    let ask = parse_decimal(msg, "a").map(Price::new);
    let volume = parse_decimal(msg, "v").map(Amount::new);
    let quote_volume = parse_decimal(msg, "q").map(Amount::new);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: None,
        high,
        low,
        bid,
        ask,
        bid_volume: None,
        ask_volume: None,
        vwap: None,
        open,
        close: last,
        last,
        previous_close: None,
        change: None,
        percentage: None,
        average: None,
        base_volume: volume,
        quote_volume,
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: msg
            .as_object()
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default(),
    })
}
