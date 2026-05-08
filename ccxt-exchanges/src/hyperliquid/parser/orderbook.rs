//! OrderBook data parser for HyperLiquid.

use ccxt_core::{
    Result,
    parser_utils::value_to_hashmap,
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

/// Parse orderbook data from HyperLiquid L2 book response.
///
/// # Note
///
/// HyperLiquid's l2Book response includes timestamp:
/// `{"coin":"BTC","time":1775942451188,"levels":[[...],[...]]}`
pub fn parse_orderbook(data: &Value, symbol: String) -> Result<OrderBook> {
    // HyperLiquid provides "time" field in l2Book response
    let timestamp = data
        .get("time")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let mut bids = Vec::new();
    let mut asks = Vec::new();

    // Parse levels
    if let Some(levels) = data["levels"].as_array() {
        if levels.len() >= 2 {
            // First array is bids, second is asks
            if let Some(bid_levels) = levels[0].as_array() {
                for level in bid_levels {
                    if let (Some(px), Some(sz)) = (
                        level["px"].as_str().and_then(|s| Decimal::from_str(s).ok()),
                        level["sz"].as_str().and_then(|s| Decimal::from_str(s).ok()),
                    ) {
                        bids.push(OrderBookEntry {
                            price: Price::new(px),
                            amount: Amount::new(sz),
                        });
                    }
                }
            }

            if let Some(ask_levels) = levels[1].as_array() {
                for level in ask_levels {
                    if let (Some(px), Some(sz)) = (
                        level["px"].as_str().and_then(|s| Decimal::from_str(s).ok()),
                        level["sz"].as_str().and_then(|s| Decimal::from_str(s).ok()),
                    ) {
                        asks.push(OrderBookEntry {
                            price: Price::new(px),
                            amount: Amount::new(sz),
                        });
                    }
                }
            }
        }
    }

    // Sort bids descending, asks ascending
    bids.sort_by(|a, b| b.price.cmp(&a.price));
    asks.sort_by(|a, b| a.price.cmp(&b.price));

    Ok(OrderBook {
        symbol: Symbol::new_unchecked(symbol),
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        nonce: None,
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
