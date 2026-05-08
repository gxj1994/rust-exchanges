//! WebSocket OrderBook data parser for OKX.

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

/// Parse orderbook from WebSocket message.
pub fn parse_orderbook(msg: &Value) -> Result<OrderBook> {
    let data = extract_first_data(msg)?;

    let arg = msg
        .get("arg")
        .ok_or_else(|| Error::invalid_request("Missing arg in orderbook message"))?;
    let inst_id = arg
        .get("instId")
        .and_then(|i| i.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(inst_id));

    let timestamp = data
        .get("ts")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    let parse_orders = |arr: &[Value]| -> Vec<OrderBookEntry> {
        arr.iter()
            .filter_map(|entry| {
                let arr = entry.as_array()?;
                if arr.len() < 2 {
                    return None;
                }
                let price = arr[0].as_str().and_then(|s| Decimal::from_str(s).ok())?;
                let amount = arr[1].as_str().and_then(|s| Decimal::from_str(s).ok())?;
                Some(OrderBookEntry {
                    price: Price::new(price),
                    amount: Amount::new(amount),
                })
            })
            .collect()
    };

    let bids = data
        .get("bids")
        .and_then(|b| b.as_array())
        .map(|arr| parse_orders(arr))
        .unwrap_or_default();

    let asks = data
        .get("asks")
        .and_then(|a| a.as_array())
        .map(|arr| parse_orders(arr))
        .unwrap_or_default();

    let mut orderbook = OrderBook::new(symbol, timestamp);
    orderbook.bids = bids;
    orderbook.asks = asks;
    Ok(orderbook)
}
