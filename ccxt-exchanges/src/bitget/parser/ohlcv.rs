//! OHLCV (candlestick) data parser for Bitget.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::OHLCV,
};
use serde_json::Value;

/// Parse OHLCV (candlestick) data from Bitget kline response.
///
/// # Arguments
///
/// * `data` - Bitget OHLCV data JSON array
///
/// # Returns
///
/// Returns a CCXT [`OHLCV`] structure.
pub fn parse_ohlcv(data: &Value) -> Result<OHLCV> {
    // Bitget V3 API 返回 OHLCV 数组: [timestamp, open, high, low, close, volume, quoteVolume]
    let arr = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_format("data", "OHLCV array")))?;

    if arr.len() < 6 {
        return Err(Error::from(ParseError::invalid_format(
            "data",
            "OHLCV array with at least 6 elements",
        )));
    }

    let timestamp = arr[0]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| arr[0].as_i64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "timestamp")))?;

    let open = arr[1]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[1].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "open")))?;

    let high = arr[2]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[2].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "high")))?;

    let low = arr[3]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[3].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "low")))?;

    let close = arr[4]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[4].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "close")))?;

    let volume = arr[5]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| arr[5].as_f64())
        .ok_or_else(|| Error::from(ParseError::invalid_value("data", "volume")))?;

    Ok(OHLCV {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    })
}
