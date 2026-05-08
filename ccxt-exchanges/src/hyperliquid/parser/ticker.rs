//! Ticker data parser for HyperLiquid.

use ccxt_core::{
    Result,
    types::{Symbol, Ticker, financial::Price},
};
use rust_decimal::Decimal;
use std::collections::HashMap;

use super::timestamp_to_datetime;

/// Parse ticker data from HyperLiquid all_mids response.
///
/// # Arguments
///
/// * `symbol` - Trading pair symbol
/// * `mid_price` - Current mid price
/// * `_market` - Optional market info (unused)
///
/// # Note
///
/// HyperLiquid's all_mids response does NOT include a timestamp:
/// `{ "BTC": "50000", "ETH": "3000", ... }`
/// So local time is used as the ticker timestamp.
pub fn parse_ticker(
    symbol: &str,
    mid_price: Decimal,
    _market: Option<&ccxt_core::types::Market>,
) -> Result<Ticker> {
    // allMids response has no timestamp, use local time
    let timestamp = chrono::Utc::now().timestamp_millis();

    Ok(Ticker {
        symbol: Symbol::new_unchecked(symbol),
        timestamp,
        datetime: timestamp_to_datetime(timestamp),
        high: None,
        low: None,
        bid: Some(Price::new(mid_price)),
        bid_volume: None,
        ask: Some(Price::new(mid_price)),
        ask_volume: None,
        vwap: None,
        open: None,
        close: Some(Price::new(mid_price)),
        last: Some(Price::new(mid_price)),
        previous_close: None,
        change: None,
        percentage: None,
        average: None,
        base_volume: None,
        quote_volume: None,
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: HashMap::new(),
    })
}
