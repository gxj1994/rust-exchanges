//! Gate.io data parser module.
//!
//! Converts Gate.io API response data into standardized CCXT format structures.
//!
//! # Design Principles
//!
//! - **Unified Parsers**: REST and WebSocket parsers share the same functions where possible
//! - **Helper Functions**: Common parsing logic extracted into reusable helpers
//! - **Type Safety**: All parsers return `Result<T>` for proper error handling

use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;
use std::collections::HashMap;

// Re-export all parser functions
pub use account::{parse_balance, parse_contract_position, parse_futures_balance};
pub use bidask::parse_bids_asks;
pub use market::{parse_contract_market, parse_currency_pair, parse_ohlcv, parse_ws_ohlcv};
pub use order::{parse_contract_order, parse_contract_orders, parse_order, parse_orders};
pub use orderbook::{parse_order_book, parse_ws_orderbook};
pub use ticker::{parse_contract_ticker, parse_ticker, parse_ws_ticker};
pub use trade::{parse_trade, parse_trades, parse_ws_trade};

mod account;
mod bidask;
mod market;
mod order;
mod orderbook;
mod ticker;
mod trade;

// ============================================================================
// Helper Functions - Type Conversion
// ============================================================================

/// Parse an f64 value from JSON (supports both string and number formats).
pub(crate) fn parse_f64(data: &Value, key: &str) -> Option<f64> {
    data.get(key).and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
    })
}

/// Parse a `Decimal` value from JSON (supports both string and number formats).
/// Prioritizes string parsing for maximum precision, falls back to f64 only when necessary.
pub(crate) fn parse_decimal(data: &Value, key: &str) -> Option<Decimal> {
    data.get(key).and_then(|v| {
        // Prioritize string parsing for maximum precision
        if let Some(s) = v.as_str() {
            Decimal::from_str(s).ok()
        } else if let Some(num) = v.as_f64() {
            // Fallback to f64 only when value is a JSON number
            Decimal::from_f64(num)
        } else {
            None
        }
    })
}

/// Parse a `Decimal` value from JSON, trying multiple keys in order.
#[allow(unused)]
pub(crate) fn parse_decimal_multi(data: &Value, keys: &[&str]) -> Option<Decimal> {
    keys.iter().find_map(|key| parse_decimal(data, key))
}

/// Convert a JSON value to a HashMap for the `info` field.
#[allow(unused)]
pub(crate) fn value_to_hashmap(value: &Value) -> HashMap<String, String> {
    let mut map = HashMap::new();

    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            map.insert(key.clone(), val.to_string());
        }
    }

    map
}

/// Parse an integer from JSON (supports both string and number formats).
#[allow(unused)]
pub(crate) fn parse_i64(data: &Value, key: &str) -> Option<i64> {
    data.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
    })
}

/// Parse a u64 timestamp (milliseconds) from JSON.
#[allow(unused)]
pub(crate) fn parse_timestamp(data: &Value, key: &str) -> Option<i64> {
    parse_i64(data, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_decimal_from_string() {
        let data = serde_json::json!({
            "price": "123.456"
        });

        let result = parse_decimal(&data, "price");
        assert_eq!(result, Some(Decimal::from_str("123.456").unwrap()));
    }

    #[test]
    fn test_parse_decimal_from_number() {
        let data = serde_json::json!({
            "price": 123.456
        });

        let result = parse_decimal(&data, "price");
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_decimal_missing() {
        let data = serde_json::json!({
            "other": "value"
        });

        let result = parse_decimal(&data, "price");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_decimal_multi() {
        let data = serde_json::json!({
            "last": "100",
            "price": "200"
        });

        let result = parse_decimal_multi(&data, &["last", "price"]);
        assert_eq!(result, Some(Decimal::from_str("100").unwrap()));
    }

    #[test]
    fn test_parse_f64() {
        let data = serde_json::json!({
            "value": "123.45"
        });

        let result = parse_f64(&data, "value");
        assert!((result.unwrap() - 123.45).abs() < 0.001);
    }

    #[test]
    fn test_parse_i64() {
        let data = serde_json::json!({
            "timestamp": "1234567890"
        });

        let result = parse_i64(&data, "timestamp");
        assert_eq!(result, Some(1234567890));
    }

    #[test]
    fn test_value_to_hashmap() {
        let data = serde_json::json!({
            "key1": "value1",
            "key2": 123
        });

        let result = value_to_hashmap(&data);
        assert_eq!(result.get("key1"), Some(&"\"value1\"".to_string()));
        assert_eq!(result.get("key2"), Some(&"123".to_string()));
    }
}
