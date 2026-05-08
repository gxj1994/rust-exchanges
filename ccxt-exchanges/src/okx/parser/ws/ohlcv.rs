//! WebSocket OHLCV data parser for OKX.

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

/// Parse OHLCV from WebSocket message.
///
/// OKX snapshot data is a 2D array: [[ts, o, h, l, c, vol, volCcy, volCcyQuote, confirm], ...]
pub fn parse_ohlcv(msg: &Value) -> Result<Vec<Ohlcv>> {
    let data_array = extract_data(msg)?;

    let mut ohlcvs = Vec::new();

    for data in data_array {
        let arr = match data.as_array() {
            Some(arr) if arr.len() >= 6 => arr,
            _ => continue,
        };

        let timestamp = arr[0]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);

        let open = arr[1]
            .as_str()
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let high = arr[2]
            .as_str()
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let low = arr[3]
            .as_str()
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let close = arr[4]
            .as_str()
            .and_then(|s| Decimal::from_str(s).ok())
            .map(Price::new)
            .unwrap_or_else(|| Price::new(Decimal::ZERO));

        let volume = arr[5]
            .as_str()
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
