//! OHLCV parser for Bybit.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::parse_timestamp,
    types::OHLCV,
};
use serde_json::Value;

/// Parse OHLCV (candlestick) data from Bybit kline response.
///
/// # Arguments
///
/// * `data` - Bybit OHLCV data JSON array or object
///
/// # Returns
///
/// Returns a CCXT [`OHLCV`] structure.
pub fn parse_ohlcv(data: &Value) -> Result<OHLCV> {
    // Bybit returns OHLCV as array: [startTime, openPrice, highPrice, lowPrice, closePrice, volume, turnover]
    // or as object with named fields
    if let Some(arr) = data.as_array() {
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
    } else {
        // Handle object format
        // Bybit WebSocket kline uses "start" field, REST uses "startTime"
        let timestamp = parse_timestamp(data, "start")
            .or_else(|| parse_timestamp(data, "startTime"))
            .or_else(|| parse_timestamp(data, "openTime"))
            .ok_or_else(|| Error::from(ParseError::missing_field("startTime")))?;

        let open = data["openPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["openPrice"].as_f64())
            .or_else(|| data["open"].as_str().and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| data["open"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("openPrice")))?;

        let high = data["highPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["highPrice"].as_f64())
            .or_else(|| data["high"].as_str().and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| data["high"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("highPrice")))?;

        let low = data["lowPrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["lowPrice"].as_f64())
            .or_else(|| data["low"].as_str().and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| data["low"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("lowPrice")))?;

        let close = data["closePrice"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["closePrice"].as_f64())
            .or_else(|| data["close"].as_str().and_then(|s| s.parse::<f64>().ok()))
            .or_else(|| data["close"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("closePrice")))?;

        let volume = data["volume"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| data["volume"].as_f64())
            .ok_or_else(|| Error::from(ParseError::missing_field("volume")))?;

        Ok(OHLCV {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_ohlcv_array() {
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

    #[test]
    fn test_parse_ohlcv_object() {
        let data = json!({
            "startTime": "1700000000000",
            "openPrice": "50000.00",
            "highPrice": "51000.00",
            "lowPrice": "49000.00",
            "closePrice": "50500.00",
            "volume": "1000.5"
        });

        let ohlcv = parse_ohlcv(&data).unwrap();
        assert_eq!(ohlcv.timestamp, 1700000000000);
        assert_eq!(ohlcv.open, 50000.00);
        assert_eq!(ohlcv.high, 51000.00);
        assert_eq!(ohlcv.low, 49000.00);
        assert_eq!(ohlcv.close, 50500.00);
        assert_eq!(ohlcv.volume, 1000.5);
    }
}
