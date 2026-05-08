//! WebSocket OHLCV data parser for Bitget.

use ccxt_core::{
    error::{Error, Result},
    types::{
        Ohlcv,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

use super::extract_data;

/// Parse OHLCV from WebSocket candle message (V3 UTA format).
///
/// V3 UTA data format is an array of objects (not 2D array like V2):
/// [{"start": "...", "open": "...", "high": "...", "low": "...", "close": "...", "volume": "...", "turnover": "..."}]
pub fn parse_ohlcv(msg: &Value) -> Result<Vec<Ohlcv>> {
    let data_array = extract_data(msg)?;

    let mut ohlcvs = Vec::new();

    for data in data_array {
        // V3 UTA: data is an object, not an array
        let timestamp = data
            .get("start")
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let open = data
            .get("open")
            .and_then(|s| s.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let high = data
            .get("high")
            .and_then(|s| s.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let low = data
            .get("low")
            .and_then(|s| s.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let close = data
            .get("close")
            .and_then(|s| s.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let volume = data
            .get("volume")
            .and_then(|s| s.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Amount::new)
            .unwrap_or_else(|| Amount::new(Decimal::ZERO));

        ohlcvs.push(Ohlcv::new(timestamp, open, high, low, close, volume));
    }

    if ohlcvs.is_empty() {
        return Err(Error::invalid_request("Kline data array is empty"));
    }

    Ok(ohlcvs)
}
