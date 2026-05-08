//! OHLCV parser for OKX.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::OHLCV,
};
use serde_json::Value;

/// Parse OHLCV (candlestick) data from OKX kline response.
///
/// # Arguments
///
/// * `data` - OKX OHLCV data JSON array
///
/// # Returns
///
/// Returns a CCXT [`OHLCV`] structure.
pub fn parse_ohlcv(data: &Value) -> Result<OHLCV> {
    // OKX returns OHLCV as array: [ts, o, h, l, c, vol, volCcy, volCcyQuote, confirm]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_ohlcv() {
        let data = json!([
            "1700000000000",
            "50000.00",
            "51000.00",
            "49000.00",
            "50500.00",
            "1000.5"
        ]);

        let ohlcv = parse_ohlcv(&data).unwrap();
        assert_eq!(ohlcv.timestamp, 1700000000000);
        assert_eq!(ohlcv.open, 50000.00);
        assert_eq!(ohlcv.high, 51000.00);
        assert_eq!(ohlcv.low, 49000.00);
        assert_eq!(ohlcv.close, 50500.00);
        assert_eq!(ohlcv.volume, 1000.5);
    }
}
