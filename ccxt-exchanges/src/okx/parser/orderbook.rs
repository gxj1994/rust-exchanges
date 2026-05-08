//! OrderBook parser for OKX.

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

/// Parse orderbook data from OKX depth response.
///
/// # Arguments
///
/// * `data` - OKX orderbook data JSON object
/// * `symbol` - Trading pair symbol
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure with bids sorted in descending order
/// and asks sorted in ascending order.
pub fn parse_orderbook(data: &Value, symbol: Symbol) -> Result<OrderBook> {
    let timestamp =
        parse_timestamp(data, "ts").unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

    let mut bids = parse_orderbook_side(&data["bids"])?;
    let mut asks = parse_orderbook_side(&data["asks"])?;

    // Sort bids in descending order (highest price first)
    bids.sort_by(|a, b| b.price.cmp(&a.price));

    // Sort asks in ascending order (lowest price first)
    asks.sort_by(|a, b| a.price.cmp(&b.price));

    Ok(OrderBook {
        symbol,
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
            // OKX format: [price, size, liquidated_orders, num_orders]
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
            "bids": [
                ["50000.00", "1.5", "0", "1"],
                ["49999.00", "2.0", "0", "2"]
            ],
            "asks": [
                ["50001.00", "1.0", "0", "1"],
                ["50002.00", "3.0", "0", "2"]
            ],
            "ts": "1700000000000"
        });

        let orderbook = parse_orderbook(&data, Symbol::new_unchecked("BTC/USDT")).unwrap();
        assert_eq!(orderbook.symbol.as_str(), "BTC/USDT");
        assert_eq!(orderbook.bids.len(), 2);
        assert_eq!(orderbook.asks.len(), 2);
        assert_eq!(orderbook.bids[0].price, Price::new(dec!(50000.00)));
        assert_eq!(orderbook.asks[0].price, Price::new(dec!(50001.00)));
    }
}
