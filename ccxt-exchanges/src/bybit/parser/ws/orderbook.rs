//! WebSocket OrderBook data parser for Bybit.

use ccxt_core::{
    error::Result,
    types::{
        OrderBook, OrderBookEntry, Symbol,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;

use super::{extract_data, to_unified_symbol};

/// Parse orderbook from WebSocket message.
pub fn parse_orderbook(msg: &Value) -> Result<OrderBook> {
    let data = extract_data(msg)?;

    let symbol_str = msg
        .get("topic")
        .and_then(|t| t.as_str())
        .and_then(|t| {
            // Extract symbol: orderbook.50.BTCUSDT -> BTCUSDT
            let parts: Vec<&str> = t.split('.').collect();
            parts.get(2).copied()
        })
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(symbol_str, Some(msg)));

    let timestamp = data.get("ts").and_then(|t| t.as_u64()).unwrap_or(0) as i64;

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
