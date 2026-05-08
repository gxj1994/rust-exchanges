//! WebSocket Ticker data parser for Bybit.

use ccxt_core::{
    error::Result,
    types::{
        Symbol, Ticker,
        financial::{Amount, Price},
    },
};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde_json::Value;
use std::collections::HashMap;

use super::{extract_data, to_unified_symbol};

/// Parse ticker from WebSocket message.
pub fn parse_ticker(msg: &Value) -> Result<Ticker> {
    let data = extract_data(msg)?;

    let symbol_str = msg
        .get("topic")
        .and_then(|t| t.as_str())
        .and_then(|t| t.strip_prefix("tickers."))
        .unwrap_or_default();

    let unified = to_unified_symbol(symbol_str, Some(msg));
    let symbol = Symbol::new_unchecked(unified);

    let last = data
        .get("lastPrice")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let bid = data
        .get("bid1Price")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let ask = data
        .get("ask1Price")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let high = data
        .get("highPrice24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let low = data
        .get("lowPrice24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let volume = data
        .get("volume24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Amount::new);

    let timestamp = data.get("ts").and_then(|t| t.as_u64()).unwrap_or(0) as i64;

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: None,
        high,
        low,
        bid,
        ask,
        bid_volume: None,
        ask_volume: None,
        vwap: None,
        open: None,
        close: last,
        last,
        previous_close: None,
        change: None,
        percentage: None,
        average: None,
        base_volume: volume,
        quote_volume: None,
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: HashMap::new(),
    })
}
