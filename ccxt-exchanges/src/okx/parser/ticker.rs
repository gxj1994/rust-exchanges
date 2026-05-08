//! Ticker parser for OKX.

use crate::common::parser_helpers::ParseHelper;
use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::financial::{Amount, Price},
    types::{Market, Symbol, Ticker},
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse ticker data from OKX ticker response.
///
/// # Arguments
///
/// * `data` - OKX ticker data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
pub fn parse_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        // Try to construct symbol from instId
        data["instId"].as_str().map_or_else(
            || Symbol::new_unchecked(""),
            |s| Symbol::new_unchecked(s.replace('-', "/")),
        )
    };

    // OKX uses "ts" for timestamp
    let timestamp = parse_timestamp(data, "ts").unwrap_or(0);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: ccxt_core::parser_utils::timestamp_to_datetime(timestamp),
        high: parse_decimal(data, "high24h").map(Price::new),
        low: parse_decimal(data, "low24h").map(Price::new),
        bid: parse_decimal(data, "bidPx").map(Price::new),
        bid_volume: parse_decimal(data, "bidSz").map(Amount::new),
        ask: parse_decimal(data, "askPx").map(Price::new),
        ask_volume: parse_decimal(data, "askSz").map(Amount::new),
        vwap: None,
        open: ParseHelper::decimal_any(data, &["open24h", "sodUtc0"]).map(Price::new),
        close: parse_decimal(data, "last").map(Price::new),
        last: parse_decimal(data, "last").map(Price::new),
        previous_close: None,
        change: None, // OKX doesn't provide direct change value
        percentage: parse_decimal(data, "sodUtc0").and_then(|open| {
            parse_decimal(data, "last").map(|last| {
                if open.is_zero() {
                    Decimal::ZERO
                } else {
                    ((last - open) / open) * Decimal::from(100)
                }
            })
        }),
        average: None,
        base_volume: ParseHelper::decimal_any(data, &["vol24h", "volCcy24h"]).map(Amount::new),
        quote_volume: parse_decimal(data, "volCcy24h").map(Amount::new),
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: value_to_hashmap(data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_ticker() {
        let data = json!({
            "instId": "BTC-USDT",
            "last": "50000.00",
            "high24h": "51000.00",
            "low24h": "49000.00",
            "bidPx": "49999.00",
            "askPx": "50001.00",
            "vol24h": "1000.5",
            "ts": "1700000000000"
        });

        let ticker = parse_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol, Symbol::new_unchecked("BTC/USDT"));
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
        assert_eq!(ticker.high, Some(Price::new(dec!(51000.00))));
        assert_eq!(ticker.low, Some(Price::new(dec!(49000.00))));
        assert_eq!(ticker.timestamp, 1700000000000);
    }
}
