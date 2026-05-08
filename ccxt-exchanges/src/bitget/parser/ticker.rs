//! Ticker data parser for Bitget.

use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::{
        Market, Symbol, Ticker,
        financial::{Amount, Price},
    },
};
use serde_json::Value;

use super::timestamp_to_datetime;

/// Parse ticker data from Bitget ticker response.
///
/// # Arguments
///
/// * `data` - Bitget ticker data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
pub fn parse_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        // Try to construct symbol from symbol field
        data["symbol"]
            .as_str()
            .map(|s| Symbol::new_unchecked(s.to_string()))
            .ok_or_else(|| {
                ccxt_core::Error::from(ccxt_core::ParseError::missing_field("symbol"))
                    .context("Failed to parse ticker: missing symbol identifier")
            })?
    };

    // Bitget V3 API 使用 "ts" 为毫秒时间戳
    let timestamp = parse_timestamp(data, "ts").unwrap_or(0);

    // 解析合约特有字段
    let funding_rate = parse_decimal(data, "fundingRate");
    let open_interest = parse_decimal(data, "openInterest");
    let index_price = parse_decimal(data, "indexPrice").map(Price::new);
    let mark_price = parse_decimal(data, "markPrice").map(Price::new);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        // V3 API 字段映射
        high: parse_decimal(data, "highPrice24h").map(Price::new),
        low: parse_decimal(data, "lowPrice24h").map(Price::new),
        bid: parse_decimal(data, "bid1Price").map(Price::new),
        bid_volume: parse_decimal(data, "bid1Size").map(Amount::new),
        ask: parse_decimal(data, "ask1Price").map(Price::new),
        ask_volume: parse_decimal(data, "ask1Size").map(Amount::new),
        vwap: None,
        open: parse_decimal(data, "openPrice24h").map(Price::new),
        close: parse_decimal(data, "lastPrice").map(Price::new),
        last: parse_decimal(data, "lastPrice").map(Price::new),
        previous_close: None,
        change: None, // V3 API 不直接提供 change，可通过 lastPrice - openPrice24h 计算
        percentage: parse_decimal(data, "price24hPcnt"),
        average: None,
        base_volume: parse_decimal(data, "volume24h").map(Amount::new),
        quote_volume: parse_decimal(data, "turnover24h").map(Amount::new),
        funding_rate,
        open_interest,
        index_price,
        mark_price,
        info: value_to_hashmap(data),
    })
}
