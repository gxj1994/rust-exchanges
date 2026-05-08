//! OHLCV (candlestick) data parser for HyperLiquid.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::parse_timestamp,
    types::{
        Ohlcv,
        financial::{Amount, Price},
    },
};
use serde_json::Value;

use super::parse_decimal_from_value;
use crate::common::parser_helpers::ParseHelper;

/// Parse OHLCV (candlestick) data from HyperLiquid candle response.
///
/// HyperLiquid returns candle data as arrays: [timestamp, open, high, low, close, volume]
///
/// # Arguments
///
/// * `data` - HyperLiquid OHLCV data JSON object or array
///
/// # Returns
///
/// Returns a CCXT [`Ohlcv`] structure.
pub fn parse_ohlcv(data: &Value) -> Result<Ohlcv> {
    // Handle array format: [timestamp, open, high, low, close, volume]
    if let Some(arr) = data.as_array() {
        if arr.len() >= 6 {
            let timestamp = arr[0]
                .as_i64()
                .or_else(|| arr[0].as_str().and_then(|s| s.parse().ok()))
                .ok_or_else(|| Error::from(ParseError::missing_field("timestamp")))?;

            let open = parse_decimal_from_value(&arr[1])
                .ok_or_else(|| Error::from(ParseError::missing_field("open")))?;
            let high = parse_decimal_from_value(&arr[2])
                .ok_or_else(|| Error::from(ParseError::missing_field("high")))?;
            let low = parse_decimal_from_value(&arr[3])
                .ok_or_else(|| Error::from(ParseError::missing_field("low")))?;
            let close = parse_decimal_from_value(&arr[4])
                .ok_or_else(|| Error::from(ParseError::missing_field("close")))?;
            let volume = parse_decimal_from_value(&arr[5])
                .ok_or_else(|| Error::from(ParseError::missing_field("volume")))?;

            return Ok(Ohlcv {
                timestamp,
                open: Price::new(open),
                high: Price::new(high),
                low: Price::new(low),
                close: Price::new(close),
                volume: Amount::new(volume),
            });
        }
    }

    // Handle object format
    let timestamp = parse_timestamp(data, "t")
        .or_else(|| parse_timestamp(data, "timestamp"))
        .ok_or_else(|| Error::from(ParseError::missing_field("timestamp")))?;

    let open = ParseHelper::decimal_any(data, &["o", "open"])
        .ok_or_else(|| Error::from(ParseError::missing_field("open")))?;
    let high = ParseHelper::decimal_any(data, &["h", "high"])
        .ok_or_else(|| Error::from(ParseError::missing_field("high")))?;
    let low = ParseHelper::decimal_any(data, &["l", "low"])
        .ok_or_else(|| Error::from(ParseError::missing_field("low")))?;
    let close = ParseHelper::decimal_any(data, &["c", "close"])
        .ok_or_else(|| Error::from(ParseError::missing_field("close")))?;
    let volume = ParseHelper::decimal_any(data, &["v", "volume"])
        .ok_or_else(|| Error::from(ParseError::missing_field("volume")))?;

    Ok(Ohlcv {
        timestamp,
        open: Price::new(open),
        high: Price::new(high),
        low: Price::new(low),
        close: Price::new(close),
        volume: Amount::new(volume),
    })
}
