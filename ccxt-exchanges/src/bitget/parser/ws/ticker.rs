//! WebSocket Ticker data parser for Bitget.

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

use super::{extract_first_data, to_unified_symbol};

/// Parse ticker from WebSocket message (V3 UTA format).
pub fn parse_ticker(msg: &Value) -> Result<Ticker> {
    let data = extract_first_data(msg)?;

    // V3 UTA: symbol在arg对象中，不在data数组中
    let arg = msg
        .get("arg")
        .ok_or_else(|| ccxt_core::error::Error::invalid_request("Missing arg in ticker message"))?;
    let inst_id = arg
        .get("symbol")
        .and_then(|i| i.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(inst_id, Some(msg)));

    // V3 UTA字段: lastPrice (不再有last/lastPr fallback)
    let last = data
        .get("lastPrice")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    // V3 UTA字段: bid1Price
    let bid = data
        .get("bid1Price")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    // V3 UTA字段: ask1Price
    let ask = data
        .get("ask1Price")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    // V3 UTA字段: high24h
    let high = data
        .get("high24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    // V3 UTA字段: low24h
    let low = data
        .get("low24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    // V3 UTA字段: volume24h
    let volume = data
        .get("volume24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Amount::new);

    // V3 UTA新增字段: bid1Size, ask1Size
    let bid_volume = data
        .get("bid1Size")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Amount::new);

    let ask_volume = data
        .get("ask1Size")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Amount::new);

    // V3 UTA新增字段: price24hPcnt (涨跌幅)
    let percentage = data
        .get("price24hPcnt")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok());

    // V3 UTA新增字段: openPrice24h
    let open = data
        .get("openPrice24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    // V3 UTA新增字段: turnover24h (成交额)
    let quote_volume = data
        .get("turnover24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Amount::new);

    let timestamp = data
        .get("ts")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| data.get("ts").and_then(|v| v.as_u64()).map(|v| v as i64))
        .unwrap_or(0);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: None,
        high,
        low,
        bid,
        ask,
        bid_volume,
        ask_volume,
        vwap: None,
        open,
        close: last,
        last,
        previous_close: None,
        change: None,
        percentage,
        average: None,
        base_volume: volume,
        quote_volume,
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: HashMap::new(),
    })
}
