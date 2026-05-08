//! OrderBook data parser for Bitget.

use ccxt_core::{
    Result,
    error::{Error, ParseError},
    parser_utils::{parse_timestamp, value_to_hashmap},
    types::{
        OrderBook, OrderBookEntry, Symbol,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};

use super::timestamp_to_datetime;

/// Parse orderbook data from Bitget depth response.
///
/// # Arguments
///
/// * `data` - Bitget orderbook data JSON object
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure with bids sorted in descending order
/// and asks sorted in ascending order.
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    let timestamp = parse_timestamp(data, "ts")
        .or_else(|| parse_timestamp(data, "timestamp"))
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    // Bitget V3 API 使用 "b" 表示买盘（bids），"a" 表示卖盘（asks）
    let bids_data = data
        .get("b")
        .or_else(|| data.get("bids"))
        .unwrap_or(&Value::Null);
    let asks_data = data
        .get("a")
        .or_else(|| data.get("asks"))
        .unwrap_or(&Value::Null);

    let mut bids = parse_orderbook_side(bids_data)?;
    let mut asks = parse_orderbook_side(asks_data)?;

    // Sort bids in descending order (highest price first)
    bids.sort_by(|a, b| b.price.cmp(&a.price));

    // Sort asks in ascending order (lowest price first)
    asks.sort_by(|a, b| a.price.cmp(&b.price));

    Ok(OrderBook {
        symbol: Symbol::new_unchecked(symbol),
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        nonce: parse_timestamp(data, "seqId"),
        bids,
        asks,
        buffered_deltas: VecDeque::new(),
        bids_map: BTreeMap::new(),
        asks_map: BTreeMap::new(),
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
