//! Advanced parser helpers for exchange API responses.
//!
//! This module provides high-level parsing utilities that reduce code duplication
//! across exchange implementations. It builds on top of `ccxt_core::parser_utils`
//! and adds:
//!
//! - Multi-key fallback parsing
//! - Safe parsing with defaults
//! - Common pattern helpers (amount, price, timestamp, etc.)
//! - Validation utilities
//!
//! # Example
//!
//! ```rust
//! use ccxt_exchanges::common::parser_helpers::{ParseHelper, parse_amount_safe};
//! use serde_json::json;
//! use std::str::FromStr;
//!
//! let data = json!({
//!     "size": "1.5",
//!     "amount": "2.0"
//! });
//!
//! // Try multiple keys in order
//! let amount = ParseHelper::decimal_any(&data, &["amount", "size", "qty"]);
//! assert_eq!(amount, Some(rust_decimal::Decimal::from_str("2.0").unwrap()));
//!
//! // Safe parsing with default
//! let amount = parse_amount_safe(&data, "size", rust_decimal::Decimal::ZERO);
//! assert_eq!(amount, rust_decimal::Decimal::from_str("1.5").unwrap());
//! ```

use ccxt_core::parser_utils::{parse_decimal, parse_timestamp};
use rust_decimal::Decimal;
use rust_decimal::prelude::{FromPrimitive, FromStr};
use serde_json::Value;

// ============================================================================
// ParseHelper - Multi-key and fallback parsing
// ============================================================================

/// Helper for parsing JSON values with fallback and validation support.
pub struct ParseHelper;

impl ParseHelper {
    /// Try to parse a `Decimal` from multiple possible keys (first match wins).
    ///
    /// # Arguments
    ///
    /// * `data` - JSON object to parse from
    /// * `keys` - List of possible field names to try
    ///
    /// # Returns
    ///
    /// The first successfully parsed value, or `None` if all keys fail.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ccxt_exchanges::common::parser_helpers::ParseHelper;
    /// use serde_json::json;
    ///
    /// let data = json!({"size": "1.5", "amount": "2.0"});
    /// let amount = ParseHelper::decimal_any(&data, &["amount", "size"]);
    /// // Returns Some(2.0) because "amount" is tried first
    /// ```
    pub fn decimal_any(data: &Value, keys: &[&str]) -> Option<Decimal> {
        for key in keys {
            if let Some(value) = parse_decimal(data, key) {
                return Some(value);
            }
        }
        None
    }

    /// Try to parse a timestamp from multiple possible keys (first match wins).
    ///
    /// # Arguments
    ///
    /// * `data` - JSON object to parse from
    /// * `keys` - List of possible field names to try
    ///
    /// # Returns
    ///
    /// The first successfully parsed timestamp, or `None` if all keys fail.
    pub fn timestamp_any(data: &Value, keys: &[&str]) -> Option<i64> {
        for key in keys {
            if let Some(ts) = parse_timestamp(data, key) {
                return Some(ts);
            }
        }
        None
    }

    /// Parse a string field with fallback to multiple keys.
    ///
    /// # Arguments
    ///
    /// * `data` - JSON object to parse from
    /// * `keys` - List of possible field names to try
    ///
    /// # Returns
    ///
    /// The first successfully parsed string, or `None` if all keys fail.
    pub fn string_any(data: &Value, keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(s) = data.get(*key).and_then(|v| v.as_str()) {
                return Some(s.to_string());
            }
        }
        None
    }

    /// Parse a `Decimal` with a default value.
    ///
    /// # Arguments
    ///
    /// * `data` - JSON object to parse from
    /// * `key` - Field name
    /// * `default` - Default value if parsing fails
    ///
    /// # Returns
    ///
    /// The parsed value or the default.
    pub fn decimal_or(data: &Value, key: &str, default: Decimal) -> Decimal {
        parse_decimal(data, key).unwrap_or(default)
    }

    /// Parse a timestamp with a default value.
    ///
    /// # Arguments
    ///
    /// * `data` - JSON object to parse from
    /// * `key` - Field name
    /// * `default` - Default value if parsing fails
    ///
    /// # Returns
    ///
    /// The parsed timestamp or the default.
    pub fn timestamp_or(data: &Value, key: &str, default: i64) -> i64 {
        parse_timestamp(data, key).unwrap_or(default)
    }

    /// Parse a boolean field with multiple possible representations.
    ///
    /// Handles:
    /// - Native JSON booleans
    /// - String "true"/"false" (case-insensitive)
    /// - Numeric 1/0
    ///
    /// # Arguments
    ///
    /// * `data` - JSON object to parse from
    /// * `key` - Field name
    ///
    /// # Returns
    ///
    /// The parsed boolean, or `false` if parsing fails.
    pub fn bool_safe(data: &Value, key: &str) -> bool {
        data.get(key).is_some_and(|v| {
            if let Some(b) = v.as_bool() {
                b
            } else if let Some(s) = v.as_str() {
                s.to_lowercase() == "true" || s == "1"
            } else if let Some(n) = v.as_u64() {
                n == 1
            } else {
                false
            }
        })
    }
}

// ============================================================================
// Safe Parsing Functions - With defaults
// ============================================================================

/// Parse amount with safe default.
///
/// # Arguments
///
/// * `data` - JSON object
/// * `key` - Field name
/// * `default` - Default amount
///
/// # Returns
///
/// The parsed amount or default.
#[must_use]
pub fn parse_amount_safe(data: &Value, key: &str, default: Decimal) -> Decimal {
    parse_decimal(data, key).unwrap_or(default)
}

/// Parse price with safe default.
///
/// # Arguments
///
/// * `data` - JSON object
/// * `key` - Field name
/// * `default` - Default price
///
/// # Returns
///
/// The parsed price or default.
#[must_use]
pub fn parse_price_safe(data: &Value, key: &str, default: Decimal) -> Decimal {
    parse_decimal(data, key).unwrap_or(default)
}

/// Parse timestamp with safe default.
///
/// # Arguments
///
/// * `data` - JSON object
/// * `key` - Field name
/// * `default` - Default timestamp
///
/// # Returns
///
/// The parsed timestamp or default.
#[must_use]
pub fn parse_timestamp_safe(data: &Value, key: &str, default: i64) -> i64 {
    parse_timestamp(data, key).unwrap_or(default)
}

// ============================================================================
// Validation Helpers
// ============================================================================

/// Validate that a JSON value is a valid decimal string.
///
/// # Arguments
///
/// * `value` - JSON value to validate
///
/// # Returns
///
/// `true` if the value can be parsed as a valid decimal.
#[must_use]
pub fn is_valid_decimal(value: &Value) -> bool {
    if let Some(s) = value.as_str() {
        Decimal::from_str(s).is_ok()
    } else if value.as_f64().is_some() {
        true
    } else {
        false
    }
}

/// Validate that a timestamp is reasonable (not too far in the past or future).
///
/// # Arguments
///
/// * `timestamp` - Timestamp in milliseconds
/// * `max_age_days` - Maximum age in days
///
/// # Returns
///
/// `true` if the timestamp is within acceptable range.
#[must_use]
pub fn is_valid_timestamp(timestamp: i64, max_age_days: i64) -> bool {
    let now = chrono::Utc::now().timestamp_millis();
    let max_age_ms = max_age_days * 24 * 60 * 60 * 1000;

    timestamp > 0 && (now - timestamp).abs() < max_age_ms
}

/// Validate that an amount is positive and reasonable.
///
/// # Arguments
///
/// * `amount` - Amount to validate
/// * `max_amount` - Maximum allowed amount
///
/// # Returns
///
/// `true` if the amount is valid.
#[must_use]
pub fn is_valid_amount(amount: Decimal, max_amount: Decimal) -> bool {
    amount > Decimal::ZERO && amount <= max_amount
}

/// Validate that a price is positive and reasonable.
///
/// # Arguments
///
/// * `price` - Price to validate
/// * `max_price` - Maximum allowed price
///
/// # Returns
///
/// `true` if the price is valid.
#[must_use]
pub fn is_valid_price(price: Decimal, max_price: Decimal) -> bool {
    price > Decimal::ZERO && price <= max_price
}

// ============================================================================
// Common Field Extraction Patterns
// ============================================================================

/// Extract order book entry from JSON array [price, amount].
///
/// # Arguments
///
/// * `level` - JSON array with price and amount
///
/// # Returns
///
/// Tuple of (price, amount) if both are valid.
#[must_use]
pub fn parse_order_book_level(level: &Value) -> Option<(Decimal, Decimal)> {
    let arr = level.as_array()?;
    if arr.len() < 2 {
        return None;
    }

    let price = parse_decimal_from_value(&arr[0])?;
    let amount = parse_decimal_from_value(&arr[1])?;

    Some((price, amount))
}

/// Parse decimal from a JSON value (string or number).
///
/// # Arguments
///
/// * `value` - JSON value
///
/// # Returns
///
/// Parsed decimal or `None`.
#[must_use]
pub fn parse_decimal_from_value(value: &Value) -> Option<Decimal> {
    if let Some(num) = value.as_f64() {
        Decimal::from_f64(num)
    } else if let Some(s) = value.as_str() {
        if s.is_empty() {
            None
        } else {
            Decimal::from_str(s).ok()
        }
    } else {
        None
    }
}

/// Parse OHLCV data from array [timestamp, open, high, low, close, volume].
///
/// # Arguments
///
/// * `candle` - JSON array with OHLCV data
///
/// # Returns
///
/// Tuple of (timestamp, open, high, low, close, volume) if all fields are valid.
#[must_use]
pub fn parse_ohlcv_array(
    candle: &Value,
) -> Option<(i64, Decimal, Decimal, Decimal, Decimal, Decimal)> {
    let arr = candle.as_array()?;
    if arr.len() < 6 {
        return None;
    }

    let timestamp = parse_timestamp_from_value(&arr[0])?;
    let open = parse_decimal_from_value(&arr[1])?;
    let high = parse_decimal_from_value(&arr[2])?;
    let low = parse_decimal_from_value(&arr[3])?;
    let close = parse_decimal_from_value(&arr[4])?;
    let volume = parse_decimal_from_value(&arr[5])?;

    Some((timestamp, open, high, low, close, volume))
}

/// Parse timestamp from a JSON value (string or number).
///
/// # Arguments
///
/// * `value` - JSON value
///
/// # Returns
///
/// Parsed timestamp or `None`.
#[must_use]
pub fn parse_timestamp_from_value(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|s| s.parse::<i64>().ok()))
}

// ============================================================================
// Field Extraction Macros
// ============================================================================

/// Extract multiple decimal fields at once.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::extract_decimals;
/// use serde_json::json;
///
/// let data = json!({
///     "price": "100.5",
///     "amount": "2.5",
///     "cost": "251.25"
/// });
///
/// let (price, amount, cost) = extract_decimals!(&data, "price", "amount", "cost");
/// assert!(price.is_some());
/// assert!(amount.is_some());
/// assert!(cost.is_some());
/// ```
#[macro_export]
macro_rules! extract_decimals {
    ($data:expr, $($key:expr),+) => {
        ($(
            ccxt_core::parser_utils::parse_decimal($data, $key)
        ),+)
    };
}

/// Extract multiple timestamp fields at once.
///
/// # Example
///
/// ```rust
/// use ccxt_exchanges::extract_timestamps;
/// use serde_json::json;
///
/// let data = json!({
///     "timestamp": 1704110400000i64,
///     "updated": 1704110500000i64
/// });
///
/// let (ts, updated) = extract_timestamps!(&data, "timestamp", "updated");
/// assert!(ts.is_some());
/// assert!(updated.is_some());
/// ```
#[macro_export]
macro_rules! extract_timestamps {
    ($data:expr, $($key:expr),+) => {
        ($(
            ccxt_core::parser_utils::parse_timestamp($data, $key)
        ),+)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_decimal_any_first_match() {
        let data = json!({"size": "1.5", "amount": "2.0"});

        // First key exists
        let amount = ParseHelper::decimal_any(&data, &["amount", "size"]);
        assert_eq!(amount, Some(Decimal::from_str("2.0").unwrap()));

        // First key doesn't exist, falls back to second
        let amount = ParseHelper::decimal_any(&data, &["qty", "size"]);
        assert_eq!(amount, Some(Decimal::from_str("1.5").unwrap()));

        // No keys exist
        let amount = ParseHelper::decimal_any(&data, &["missing", "none"]);
        assert_eq!(amount, None);
    }

    #[test]
    fn test_timestamp_any() {
        let data = json!({
            "time": 1704110400000i64,
            "timestamp": "1704110500000"
        });

        let ts = ParseHelper::timestamp_any(&data, &["timestamp", "time"]);
        assert_eq!(ts, Some(1704110500000));

        let ts = ParseHelper::timestamp_any(&data, &["missing", "time"]);
        assert_eq!(ts, Some(1704110400000));
    }

    #[test]
    fn test_string_any() {
        let data = json!({"symbol": "BTCUSDT", "pair": "BTC-USDT"});

        let symbol = ParseHelper::string_any(&data, &["symbol", "pair"]);
        assert_eq!(symbol, Some("BTCUSDT".to_string()));

        let symbol = ParseHelper::string_any(&data, &["missing", "pair"]);
        assert_eq!(symbol, Some("BTC-USDT".to_string()));
    }

    #[test]
    fn test_decimal_or() {
        let data = json!({"price": "100.5"});

        let price = ParseHelper::decimal_or(&data, "price", Decimal::ZERO);
        assert_eq!(price, Decimal::from_str("100.5").unwrap());

        let price = ParseHelper::decimal_or(&data, "missing", Decimal::from_str("999.0").unwrap());
        assert_eq!(price, Decimal::from_str("999.0").unwrap());
    }

    #[test]
    fn test_bool_safe() {
        let data = json!({
            "bool_true": true,
            "bool_false": false,
            "string_true": "true",
            "string_false": "false",
            "num_one": 1,
            "num_zero": 0
        });

        assert!(ParseHelper::bool_safe(&data, "bool_true"));
        assert!(!ParseHelper::bool_safe(&data, "bool_false"));
        assert!(ParseHelper::bool_safe(&data, "string_true"));
        assert!(!ParseHelper::bool_safe(&data, "string_false"));
        assert!(ParseHelper::bool_safe(&data, "num_one"));
        assert!(!ParseHelper::bool_safe(&data, "num_zero"));
        assert!(!ParseHelper::bool_safe(&data, "missing"));
    }

    #[test]
    fn test_safe_parsing() {
        let data = json!({"amount": "1.5", "price": "100.0", "time": 1704110400000i64});

        assert_eq!(
            parse_amount_safe(&data, "amount", Decimal::ZERO),
            Decimal::from_str("1.5").unwrap()
        );
        assert_eq!(
            parse_amount_safe(&data, "missing", Decimal::ZERO),
            Decimal::ZERO
        );

        assert_eq!(
            parse_price_safe(&data, "price", Decimal::ZERO),
            Decimal::from_str("100.0").unwrap()
        );

        assert_eq!(parse_timestamp_safe(&data, "time", 0), 1704110400000);
    }

    #[test]
    fn test_is_valid_decimal() {
        assert!(is_valid_decimal(&json!("123.45")));
        assert!(is_valid_decimal(&json!(123.45)));
        assert!(!is_valid_decimal(&json!("invalid")));
        assert!(!is_valid_decimal(&json!(null)));
    }

    #[test]
    fn test_is_valid_timestamp() {
        let now = chrono::Utc::now().timestamp_millis();
        assert!(is_valid_timestamp(now, 365));
        assert!(!is_valid_timestamp(0, 365));
        assert!(!is_valid_timestamp(now - 400 * 24 * 60 * 60 * 1000, 365));
    }

    #[test]
    fn test_parse_order_book_level() {
        let level = json!(["100.5", "2.5"]);
        let result = parse_order_book_level(&level);
        assert_eq!(
            result,
            Some((
                Decimal::from_str("100.5").unwrap(),
                Decimal::from_str("2.5").unwrap()
            ))
        );

        let invalid = json!(["100.5"]);
        assert_eq!(parse_order_book_level(&invalid), None);
    }

    #[test]
    fn test_parse_ohlcv_array() {
        let candle = json!([
            1704110400000i64,
            "100.0",
            "105.0",
            "95.0",
            "102.0",
            "1000.0"
        ]);

        let result = parse_ohlcv_array(&candle);
        assert!(result.is_some());

        let (ts, open, high, low, close, vol) = result.unwrap();
        assert_eq!(ts, 1704110400000);
        assert_eq!(open, Decimal::from_str("100.0").unwrap());
        assert_eq!(high, Decimal::from_str("105.0").unwrap());
        assert_eq!(low, Decimal::from_str("95.0").unwrap());
        assert_eq!(close, Decimal::from_str("102.0").unwrap());
        assert_eq!(vol, Decimal::from_str("1000.0").unwrap());
    }

    #[test]
    fn test_extract_macros() {
        let data = json!({
            "price": "100.5",
            "amount": "2.5",
            "timestamp": 1704110400000i64
        });

        let (price, amount) = extract_decimals!(&data, "price", "amount");
        assert_eq!(price, Some(Decimal::from_str("100.5").unwrap()));
        assert_eq!(amount, Some(Decimal::from_str("2.5").unwrap()));

        let ts = extract_timestamps!(&data, "timestamp");
        assert_eq!(ts, Some(1704110400000));
    }
}
