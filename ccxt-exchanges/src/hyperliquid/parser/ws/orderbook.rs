//! WebSocket OrderBook data parser for HyperLiquid.
//!
//! Parses l2Book channel messages.

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

use crate::hyperliquid::core::symbol::HyperliquidSymbolConverter;

/// Parse orderbook from WebSocket l2Book message.
///
/// # Arguments
///
/// * `data` - The `data` field from WebSocket message (not the full message)
///
/// # Returns
///
/// Returns a CCXT [`OrderBook`] structure.
pub fn parse_orderbook(data: &Value) -> Result<OrderBook> {
    let coin = data
        .get("coin")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(HyperliquidSymbolConverter::exchange_to_unified_inferred(
        coin,
    ));

    let timestamp = data.get("time").and_then(|t| t.as_i64()).unwrap_or(0);

    let parse_levels = |levels: &[Value]| -> Vec<OrderBookEntry> {
        levels
            .iter()
            .filter_map(|level| {
                let price = level.get("px")?.as_str()?;
                let size = level.get("sz")?.as_str()?;

                let price = Decimal::from_str(price).ok()?;
                let amount = Decimal::from_str(size).ok()?;

                Some(OrderBookEntry {
                    price: Price::new(price),
                    amount: Amount::new(amount),
                })
            })
            .collect()
    };

    let bids = data
        .get("levels")
        .and_then(|l| l.as_array())
        .and_then(|arr| arr.first())
        .and_then(|level| level.as_array())
        .map(|arr| parse_levels(arr))
        .unwrap_or_default();

    let asks = data
        .get("levels")
        .and_then(|l| l.as_array())
        .and_then(|arr| arr.get(1))
        .and_then(|level| level.as_array())
        .map(|arr| parse_levels(arr))
        .unwrap_or_default();

    let mut ob = OrderBook::new(symbol, timestamp);
    ob.bids = bids;
    ob.asks = asks;
    Ok(ob)
}

/// Parse orderbook delta from WebSocket message.
///
/// For incremental updates, parses individual bid/ask levels.
#[allow(unused)]
pub fn parse_orderbook_delta(data: &Value) -> Result<(Vec<OrderBookEntry>, Vec<OrderBookEntry>)> {
    let parse_levels = |levels: &Value| -> Vec<OrderBookEntry> {
        levels
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|level| {
                        let price = level.get("px").and_then(|p| p.as_str())?;
                        let size = level.get("sz").and_then(|s| s.as_str())?;
                        let price = Decimal::from_str(price).ok()?;
                        let amount = Decimal::from_str(size).ok()?;
                        Some(OrderBookEntry {
                            price: Price::new(price),
                            amount: Amount::new(amount),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let bids = data
        .get("bids")
        .map(|l| parse_levels(l))
        .unwrap_or_default();

    let asks = data
        .get("asks")
        .map(|l| parse_levels(l))
        .unwrap_or_default();

    Ok((bids, asks))
}
