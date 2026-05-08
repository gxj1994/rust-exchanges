//! OrderBook parser for Bybit.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_timestamp, value_to_hashmap},
    types::financial::{Amount, Price},
    types::{OrderBook, OrderBookEntry, Symbol},
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

/// Parse orderbook data from Bybit depth response.
///
/// # Arguments
///
/// * `data` - Bybit orderbook data JSON object
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure with bids sorted in descending order
/// and asks sorted in ascending order.
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    parse_orderbook_with_ts(data, symbol, None)
}

/// Parse orderbook data with external timestamp.
///
/// # Arguments
///
/// * `data` - Bybit orderbook data JSON object
/// * `symbol` - Trading pair symbol
/// * `external_ts` - Optional timestamp from WebSocket outer message (`ts` field)
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure.
pub fn parse_orderbook_with_ts(
    data: &Value,
    symbol: String,
    external_ts: Option<i64>,
) -> Result<OrderBook> {
    // Use external timestamp if provided, otherwise try to extract from data
    let timestamp = external_ts.unwrap_or_else(|| {
        parse_timestamp(data, "ts")
            .or_else(|| parse_timestamp(data, "time"))
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis())
    });

    let mut bids = parse_orderbook_side(&data["b"])?;
    let mut asks = parse_orderbook_side(&data["a"])?;

    // Sort bids in descending order (highest price first)
    bids.sort_by(|a, b| b.price.cmp(&a.price));

    // Sort asks in ascending order (lowest price first)
    asks.sort_by(|a, b| a.price.cmp(&b.price));

    Ok(OrderBook {
        symbol: Symbol::new_unchecked(symbol),
        timestamp,
        datetime: ccxt_core::parser_utils::timestamp_to_datetime(timestamp),
        nonce: None,
        bids,
        asks,
        buffered_deltas: std::collections::VecDeque::new(),
        bids_map: std::collections::BTreeMap::new(),
        asks_map: std::collections::BTreeMap::new(),
        is_synced: false,
        needs_resync: false,
        last_resync_time: 0,
        info: value_to_hashmap(data),
    })
}

/// Parse one side (bids or asks) of orderbook data.
fn parse_orderbook_side(data: &Value) -> Result<Vec<OrderBookEntry>> {
    let Some(array) = data.as_array() else {
        return Ok(Vec::new());
    };

    let mut result = Vec::new();

    for item in array {
        if let Some(arr) = item.as_array() {
            // Bybit format: [price, size]
            if arr.len() >= 2 {
                let price = arr[0]
                    .as_str()
                    .and_then(|s| Decimal::from_str(s).ok())
                    .or_else(|| arr[0].as_f64().and_then(Decimal::from_f64_retain))
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "price")))?;

                let amount = arr[1]
                    .as_str()
                    .and_then(|s| Decimal::from_str(s).ok())
                    .or_else(|| arr[1].as_f64().and_then(Decimal::from_f64_retain))
                    .ok_or_else(|| Error::from(ParseError::invalid_value("data", "amount")))?;

                result.push(OrderBookEntry {
                    price: Price::new(price),
                    amount: Amount::new(amount),
                });
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_orderbook() {
        let data = json!({
            "b": [
                ["50000.00", "1.5"],
                ["49999.00", "2.0"]
            ],
            "a": [
                ["50001.00", "1.0"],
                ["50002.00", "3.0"]
            ],
            "ts": "1700000000000"
        });

        let orderbook = parse_orderbook(&data, "BTC/USDT".to_string()).unwrap();
        assert_eq!(orderbook.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);
        assert_eq!(orderbook.bids[0].price, Price::new(dec!(50000.00)));
        assert_eq!(orderbook.asks[0].price, Price::new(dec!(50001.00)));
    }
}
