//! WebSocket OHLCV data parser for Bybit.

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

/// Parse OHLCV from WebSocket kline message.
///
/// Bybit kline format: [{start, end, interval, open, close, high, low, volume, turnover, confirm, timestamp}, ...]
pub fn parse_ohlcv(msg: &Value) -> Result<Vec<Ohlcv>> {
    let data = extract_data(msg)?;

    let arr = data
        .as_array()
        .ok_or_else(|| Error::invalid_request("Kline data is not an array"))?;

    let mut ohlcvs = Vec::new();

    for item in arr {
        let timestamp = item.get("start").and_then(|t| t.as_u64()).unwrap_or(0) as i64;

        let open = item
            .get("open")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let close = item
            .get("close")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let high = item
            .get("high")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let low = item
            .get("low")
            .and_then(|v| v.as_str())
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let volume = item
            .get("volume")
            .and_then(|v| v.as_str())
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
