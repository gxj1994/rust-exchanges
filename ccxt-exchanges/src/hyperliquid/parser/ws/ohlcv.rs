//! WebSocket OHLCV data parser for HyperLiquid.
//!
//! Parses candle channel messages.

use ccxt_core::{
    error::Result,
    types::{
        Ohlcv,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;

use super::parse_decimal_from_value;

/// Parse OHLCV from WebSocket candle message.
///
/// # Arguments
///
/// * `data` - The `data` field from WebSocket message
///
/// # Returns
///
/// Returns a vector of CCXT [`Ohlcv`] structures (WebSocket typically sends single candle).
pub fn parse_ohlcv(data: &Value) -> Result<Vec<Ohlcv>> {
    // Hyperliquid K线数据格式可能有两种：
    // 1. 对象格式: {t, o, h, l, c, v, s}  (WebSocket 使用此格式)
    // 2. 数组格式: [timestamp, open, high, low, close, volume]

    // 尝试解析数组格式（快照消息可能是数组）
    if let Some(arr) = data.as_array() {
        let mut ohlcvs = Vec::new();
        for item in arr {
            if let Ok(ohlcv) = parse_ohlcv_item(item) {
                ohlcvs.push(ohlcv);
            }
        }
        if !ohlcvs.is_empty() {
            return Ok(ohlcvs);
        }
    }

    // 解析单个对象格式
    let ohlcv = parse_ohlcv_item(data)?;
    Ok(vec![ohlcv])
}

/// Parse single OHLCV item.
fn parse_ohlcv_item(data: &Value) -> Result<Ohlcv> {
    // 尝试数组格式: [timestamp, open, high, low, close, volume]
    if let Some(arr) = data.as_array() {
        if arr.len() >= 6 {
            let timestamp = arr[0]
                .as_i64()
                .or_else(|| arr[0].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(0);

            let open = parse_decimal_from_value(&arr[1])
                .map(Price::new)
                .unwrap_or_else(|| Price::new(Decimal::ZERO));

            let high = parse_decimal_from_value(&arr[2])
                .map(Price::new)
                .unwrap_or_else(|| Price::new(Decimal::ZERO));

            let low = parse_decimal_from_value(&arr[3])
                .map(Price::new)
                .unwrap_or_else(|| Price::new(Decimal::ZERO));

            let close = parse_decimal_from_value(&arr[4])
                .map(Price::new)
                .unwrap_or_else(|| Price::new(Decimal::ZERO));

            let volume = parse_decimal_from_value(&arr[5])
                .map(Amount::new)
                .unwrap_or_else(|| Amount::new(Decimal::ZERO));

            return Ok(Ohlcv::new(timestamp, open, high, low, close, volume));
        }
    }

    // 对象格式: {t, o, h, l, c, v}
    let timestamp = data
        .get("t")
        .and_then(|t| t.as_i64())
        .or_else(|| data.get("timestamp").and_then(|t| t.as_i64()))
        .unwrap_or(0);

    let open = data
        .get("o")
        .and_then(|v| parse_decimal_from_value(v))
        .or_else(|| data.get("open").and_then(|v| parse_decimal_from_value(v)))
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));

    let high = data
        .get("h")
        .and_then(|v| parse_decimal_from_value(v))
        .or_else(|| data.get("high").and_then(|v| parse_decimal_from_value(v)))
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));

    let low = data
        .get("l")
        .and_then(|v| parse_decimal_from_value(v))
        .or_else(|| data.get("low").and_then(|v| parse_decimal_from_value(v)))
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));

    let close = data
        .get("c")
        .and_then(|v| parse_decimal_from_value(v))
        .or_else(|| data.get("close").and_then(|v| parse_decimal_from_value(v)))
        .map(Price::new)
        .unwrap_or_else(|| Price::new(Decimal::ZERO));

    let volume = data
        .get("v")
        .and_then(|v| parse_decimal_from_value(v))
        .or_else(|| data.get("volume").and_then(|v| parse_decimal_from_value(v)))
        .map(Amount::new)
        .unwrap_or_else(|| Amount::new(Decimal::ZERO));

    Ok(Ohlcv::new(timestamp, open, high, low, close, volume))
}
