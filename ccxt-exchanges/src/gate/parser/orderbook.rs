//! Gate.io order book parser.
//!
//! Parses order book data from Gate.io spot and contract API responses.
//!
//! # API Response Format
//!
//! Gate.io order book response format:
//! ```json
//! {
//!   "id": 1234567890,
//!   "current": 1609917600,
//!   "update": 1609917601,
//!   "asks": [
//!     ["40001", "0.5"],
//!     ["40002", "1.0"]
//!   ],
//!   "bids": [
//!     ["40000", "0.3"],
//!     ["39999", "0.8"]
//!   ]
//! }
//! ```
//!
//! Each entry in bids/asks is an array: `[price, amount]`

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::financial::{Amount, Price},
    types::{OrderBook, OrderBookEntry, Symbol},
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;

/// Convert JSON Value to HashMap<String, Value> for the `info` field.
fn value_to_info(value: &Value) -> HashMap<String, Value> {
    let mut map = HashMap::new();

    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            map.insert(key.clone(), val.clone());
        }
    }

    map
}

/// Parse order book from Gate.io spot API response.
///
/// # Arguments
///
/// * `data` - Gate.io order book response JSON
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT OrderBook structure.
pub fn parse_order_book(data: &Value, symbol: Symbol) -> Result<OrderBook> {
    // Parse timestamp
    let timestamp = data["update"]
        .as_i64()
        .or_else(|| data["current"].as_i64())
        .map(|t| t * 1000) // Convert seconds to milliseconds
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    // Parse nonce/update ID
    let nonce = data["id"].as_i64();

    // Parse bids and asks
    let bids = parse_orderbook_side(&data["bids"])?;
    let asks = parse_orderbook_side(&data["asks"])?;

    // Format datetime
    let datetime = chrono::DateTime::from_timestamp_millis(timestamp).map(|dt| dt.to_rfc3339());

    Ok(OrderBook {
        symbol,
        timestamp,
        datetime,
        nonce,
        bids,
        asks,
        info: value_to_info(data),
        buffered_deltas: std::collections::VecDeque::new(),
        bids_map: std::collections::BTreeMap::new(),
        asks_map: std::collections::BTreeMap::new(),
        is_synced: false,
        needs_resync: false,
        last_resync_time: 0,
    })
}

/// Parse WebSocket order book update from Gate.io.
///
/// # WebSocket OrderBook Response (Spot)
///
/// ```json
/// {
///     "currency_pair": "BTC_USDT",
///     "asks": [["40001", "0.5"], ["40002", "1.0"]],
///     "bids": [["39999", "0.5"], ["39998", "1.0"]],
///     "id": 1234567890,
///     "current": 1609917600,
///     "update": 1609917601
/// }
/// ```
///
/// # WebSocket OrderBook Response (Futures)
///
/// ```json
/// {
///     "contract": "BTC_USDT",
///     "asks": [["40001", "0.5"], ["40002", "1.0"]],
///     "bids": [["39999", "0.5"], ["39998", "1.0"]],
///     "id": 1234567890,
///     "current": 1609917600,
///     "update": 1609917601
/// }
/// ```
pub fn parse_ws_orderbook(data: &Value) -> Result<OrderBook> {
    // Detect symbol from currency_pair (spot), contract (futures), or s (spot orderbook push)
    let symbol = if let Some(currency_pair) = data["currency_pair"].as_str() {
        // Spot: BTC_USDT -> BTC/USDT
        let symbol_str = currency_pair.replace('_', "/");
        Symbol::new_unchecked(&symbol_str)
    } else if let Some(contract) = data["contract"].as_str() {
        // Futures: BTC_USDT -> BTC/USDT:USDT
        let base_quote = contract.replace('_', "/");
        let unified = format!("{}:USDT", base_quote);
        Symbol::new_unchecked(&unified)
    } else if let Some(s) = data["s"].as_str() {
        // Spot orderbook push: BTC_USDT -> BTC/USDT
        let symbol_str = s.replace('_', "/");
        Symbol::new_unchecked(&symbol_str)
    } else {
        return Err(Error::from(ParseError::missing_field(
            "currency_pair, contract, or s",
        )));
    };

    // Parse timestamp
    let timestamp = data["update"]
        .as_i64()
        .or_else(|| data["current"].as_i64())
        .map(|t| t * 1000) // Convert seconds to milliseconds
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    // Parse nonce/update ID
    let nonce = data["id"].as_i64();

    // Parse bids and asks
    let bids = parse_orderbook_side(&data["bids"])?;
    let asks = parse_orderbook_side(&data["asks"])?;

    // Format datetime
    let datetime = chrono::DateTime::from_timestamp_millis(timestamp).map(|dt| dt.to_rfc3339());

    Ok(OrderBook {
        symbol,
        timestamp,
        datetime,
        nonce,
        bids,
        asks,
        info: value_to_info(data),
        buffered_deltas: std::collections::VecDeque::new(),
        bids_map: std::collections::BTreeMap::new(),
        asks_map: std::collections::BTreeMap::new(),
        is_synced: false,
        needs_resync: false,
        last_resync_time: 0,
    })
}

/// Parse one side of the order book (bids or asks).
///
/// # Arguments
///
/// * `data` - Array of [price, amount] pairs (spot) OR array of {p, s} objects (futures)
///
/// # Returns
///
/// Returns a vector of OrderBookEntry.
fn parse_orderbook_side(data: &Value) -> Result<Vec<OrderBookEntry>> {
    let entries = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

    let mut result = Vec::with_capacity(entries.len());

    for entry in entries {
        // 支持两种格式：
        // 1. 现货: ["40001", "0.5"] (数组)
        // 2. 期货: {"p": "75798.6", "s": 52816} (对象)
        
        let (price_str, amount_str) = if let Some(entry_array) = entry.as_array() {
            // 现货格式: [price, amount]
            if entry_array.len() < 2 {
                return Err(Error::from(ParseError::invalid_format(
                    "entry",
                    "Expected [price, amount] array with at least 2 elements",
                )));
            }

            let price = if let Some(s) = entry_array[0].as_str() {
                s.to_string()
            } else if let Some(f) = entry_array[0].as_f64() {
                format!("{}", f)
            } else {
                return Err(Error::from(ParseError::missing_field("price")));
            };

            let amount = if let Some(s) = entry_array[1].as_str() {
                s.to_string()
            } else if let Some(f) = entry_array[1].as_f64() {
                format!("{}", f)
            } else if let Some(n) = entry_array[1].as_i64() {
                format!("{}", n)
            } else {
                return Err(Error::from(ParseError::missing_field("amount")));
            };

            (price, amount)
        } else if let Some(entry_obj) = entry.as_object() {
            // 期货格式: {"p": price, "s": size}
            let price = entry_obj
                .get("p")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| Error::from(ParseError::missing_field("p (price)")))?;

            let amount = entry_obj
                .get("s")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    entry_obj
                        .get("s")
                        .and_then(|v| v.as_i64())
                        .map(|n| format!("{}", n))
                })
                .ok_or_else(|| Error::from(ParseError::missing_field("s (size)")))?;

            (price, amount)
        } else {
            return Err(Error::from(ParseError::invalid_format(
                "entry",
                "Expected [price, amount] array or {p, s} object",
            )));
        };

        // Parse price
        let price = price_str
            .parse::<Decimal>()
            .map_err(|e| Error::from(ParseError::invalid_format("price", e.to_string())))?;

        // Parse amount
        let amount = amount_str
            .parse::<Decimal>()
            .map_err(|e| Error::from(ParseError::invalid_format("amount", e.to_string())))?;

        result.push(OrderBookEntry {
            price: Price::new(price),
            amount: Amount::new(amount),
        });
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_orderbook_basic() {
        let data = json!({
            "id": 1234567890i64,
            "current": 1609917600i64,
            "update": 1609917601i64,
            "asks": [
                ["40001", "0.5"],
                ["40002", "1.0"]
            ],
            "bids": [
                ["40000", "0.3"],
                ["39999", "0.8"]
            ]
        });

        let symbol = Symbol::new_unchecked("BTC/USDT");
        let orderbook = parse_order_book(&data, symbol).unwrap();

        assert_eq!(orderbook.symbol.to_string(), "BTC/USDT");
        assert_eq!(orderbook.nonce, Some(1234567890));

        // Check bids
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.bids[0].price, Price::new(Decimal::new(40000, 0)));
        assert_eq!(orderbook.bids[0].amount, Amount::new(Decimal::new(3, 1)));
        assert_eq!(orderbook.bids[1].price, Price::new(Decimal::new(39999, 0)));
        assert_eq!(orderbook.bids[1].amount, Amount::new(Decimal::new(8, 1)));

        // Check asks
        assert_eq!(orderbook.asks.len(), 2);
        assert_eq!(orderbook.asks[0].price, Price::new(Decimal::new(40001, 0)));
        assert_eq!(orderbook.asks[0].amount, Amount::new(Decimal::new(5, 1)));
    }

    #[test]
    fn test_parse_orderbook_empty() {
        let data = json!({
            "id": 1111111111i64,
            "current": 1609917600i64,
            "update": 1609917601i64,
            "asks": [],
            "bids": []
        });

        let symbol = Symbol::new_unchecked("ETH/USDT");
        let orderbook = parse_order_book(&data, symbol).unwrap();

        assert_eq!(orderbook.bids.len(), 0);
        assert_eq!(orderbook.asks.len(), 0);
    }

    #[test]
    fn test_parse_orderbook_numeric_values() {
        // Test with numeric price/amount instead of strings
        let data = json!({
            "id": 2222222222i64,
            "current": 1609917600i64,
            "update": 1609917601i64,
            "asks": [
                [50000.5, 0.25],
                [50001.0, 0.5]
            ],
            "bids": [
                [50000.0, 0.1]
            ]
        });

        let symbol = Symbol::new_unchecked("BTC/USDT");
        let orderbook = parse_order_book(&data, symbol).unwrap();

        assert_eq!(orderbook.bids.len(), 1);
        assert_eq!(orderbook.asks.len(), 2);

        // Verify numeric values are parsed correctly
        assert!(orderbook.bids[0].price > Price::new(Decimal::new(49999, 0)));
        assert!(orderbook.bids[0].amount > Amount::new(Decimal::new(1, 2)));
    }

    #[test]
    fn test_parse_orderbook_timestamp_conversion() {
        let data = json!({
            "id": 3333333333i64,
            "current": 1609917600i64,
            "update": 1609917601i64,
            "asks": [["40000", "0.1"]],
            "bids": [["39999", "0.1"]]
        });

        let symbol = Symbol::new_unchecked("BTC/USDT");
        let orderbook = parse_order_book(&data, symbol).unwrap();

        // Should convert seconds to milliseconds
        assert_eq!(orderbook.timestamp, 1609917601000);
        assert!(orderbook.datetime.is_some());
    }
}
