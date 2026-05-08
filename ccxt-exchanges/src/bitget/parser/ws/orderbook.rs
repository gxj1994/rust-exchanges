//! WebSocket OrderBook data parser for Bitget.

use ccxt_core::{
    error::{Error, Result},
    types::{
        OrderBook, OrderBookEntry, Symbol,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

use super::{extract_first_data, to_unified_symbol};

/// Parse orderbook from WebSocket message (V3 UTA format).
pub fn parse_orderbook(msg: &Value) -> Result<OrderBook> {
    let data = extract_first_data(msg)?;

    let arg = msg
        .get("arg")
        .ok_or_else(|| Error::invalid_request("Missing arg in orderbook message"))?;
    let inst_id = arg
        .get("symbol")
        .and_then(|i| i.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(inst_id, Some(msg)));

    let timestamp = data
        .get("ts")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let parse_orders = |arr: &[Value]| -> Vec<OrderBookEntry> {
        arr.iter()
            .filter_map(|entry| {
                let price = entry.get(0)?.as_str()?;
                let amount = entry.get(1)?.as_str()?;

                let price = Decimal::from_str(price).ok()?;
                let amount = Decimal::from_str(amount).ok()?;

                Some(OrderBookEntry {
                    price: Price::new(price),
                    amount: Amount::new(amount),
                })
            })
            .collect()
    };

    // V3 UTA字段: b (bids), a (asks)
    let bids = data
        .get("b")
        .and_then(|b| b.as_array())
        .map(|arr| parse_orders(arr))
        .unwrap_or_default();

    let asks = data
        .get("a")
        .and_then(|a| a.as_array())
        .map(|arr| parse_orders(arr))
        .unwrap_or_default();

    let mut ob = OrderBook::new(symbol, timestamp);
    ob.bids = bids;
    ob.asks = asks;
    Ok(ob)
}
