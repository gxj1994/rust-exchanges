//! WebSocket OHLCV data parser for Binance.
//!
//! Parses kline events.

use ccxt_core::{
    error::{Error, Result},
    types::{
        Ohlcv,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;

use super::parse_decimal;

/// Parse OHLCV from WebSocket kline message.
///
/// # Arguments
///
/// * `msg` - The full WebSocket message
///
/// # Returns
///
/// Returns a vector of CCXT [`Ohlcv`] structures (single candle wrapped in array).
pub fn parse_ohlcv(msg: &Value) -> Result<Vec<Ohlcv>> {
    let kline = msg
        .get("k")
        .ok_or_else(|| Error::invalid_request("Missing kline data in message"))?;

    let timestamp = kline.get("t").and_then(|t| t.as_i64()).unwrap_or(0);

    let open = parse_decimal(kline, "o")
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));
    let high = parse_decimal(kline, "h")
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));
    let low = parse_decimal(kline, "l")
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));
    let close = parse_decimal(kline, "c")
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));
    let volume = parse_decimal(kline, "v")
        .map(Amount::new)
        .unwrap_or_else(|| Amount::new(Decimal::ZERO));

    // Binance pushes single candle, wrap in array
    Ok(vec![Ohlcv::new(timestamp, open, high, low, close, volume)])
}
