//! WebSocket Ticker data parser for OKX.

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

/// Parse ticker from WebSocket message.
pub fn parse_ticker(msg: &Value) -> Result<Ticker> {
    let data = extract_first_data(msg)?;

    let inst_id = data
        .get("instId")
        .and_then(|i| i.as_str())
        .unwrap_or_default();
    let symbol = Symbol::new_unchecked(to_unified_symbol(inst_id));

    let last = data
        .get("last")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let bid = data
        .get("bidPx")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let ask = data
        .get("askPx")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let high = data
        .get("high24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let low = data
        .get("low24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Price::new);

    let volume = data
        .get("vol24h")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Amount::new);

    // Parse bid/ask volumes (bidSz/askSz)
    let bid_volume = data
        .get("bidSz")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Amount::new);

    let ask_volume = data
        .get("askSz")
        .and_then(|v| v.as_str())
        .and_then(|s| Decimal::from_str(s).ok())
        .map(Amount::new);

    let timestamp = data
        .get("ts")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: None,
        high,
        low,
        bid,
        bid_volume,
        ask,
        ask_volume,
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    #[test]
    fn test_parse_ws_ticker() {
        let msg = json!({
            "arg": {
                "channel": "tickers",
                "instId": "BTC-USDT"
            },
            "data": [{
                "instId": "BTC-USDT",
                "last": "50000.00",
                "lastSz": "0.001",
                "askPx": "50001.00",
                "askSz": "2.5",
                "bidPx": "49999.00",
                "bidSz": "1.8",
                "high24h": "51000.00",
                "low24h": "49000.00",
                "vol24h": "1000.5",
                "ts": "1700000000000"
            }]
        });

        let ticker = parse_ticker(&msg).unwrap();

        // Verify symbol (BTC-USDT -> BTC/USDT for spot)
        assert_eq!(ticker.symbol.as_str(), "BTC/USDT");

        // Verify prices
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
        assert_eq!(ticker.bid, Some(Price::new(dec!(49999.00))));
        assert_eq!(ticker.ask, Some(Price::new(dec!(50001.00))));
        assert_eq!(ticker.high, Some(Price::new(dec!(51000.00))));
        assert_eq!(ticker.low, Some(Price::new(dec!(49000.00))));

        // Verify volumes (NEW: bid_volume and ask_volume should be parsed)
        assert_eq!(ticker.bid_volume, Some(Amount::new(dec!(1.8))));
        assert_eq!(ticker.ask_volume, Some(Amount::new(dec!(2.5))));
        assert_eq!(ticker.base_volume, Some(Amount::new(dec!(1000.5))));

        // Verify timestamp
        assert_eq!(ticker.timestamp, 1700000000000);
    }

    #[test]
    fn test_parse_ws_ticker_zero_volume() {
        // Test when bidSz/askSz is "0" (no volume)
        let msg = json!({
            "arg": {
                "channel": "tickers",
                "instId": "BTC-USDT"
            },
            "data": [{
                "instId": "BTC-USDT",
                "last": "50000.00",
                "lastSz": "0",
                "askPx": "50001.00",
                "askSz": "0",
                "bidPx": "49999.00",
                "bidSz": "0",
                "high24h": "51000.00",
                "low24h": "49000.00",
                "vol24h": "1000.5",
                "ts": "1700000000000"
            }]
        });

        let ticker = parse_ticker(&msg).unwrap();

        // Volumes should be Some(0), not None
        assert_eq!(ticker.bid_volume, Some(Amount::new(dec!(0))));
        assert_eq!(ticker.ask_volume, Some(Amount::new(dec!(0))));
    }

    #[test]
    fn test_parse_ws_ticker_missing_fields() {
        // Test when bidSz/askSz are missing
        let msg = json!({
            "arg": {
                "channel": "tickers",
                "instId": "BTC-USDT"
            },
            "data": [{
                "instId": "BTC-USDT",
                "last": "50000.00",
                "askPx": "50001.00",
                "bidPx": "49999.00",
                "high24h": "51000.00",
                "low24h": "49000.00",
                "vol24h": "1000.5",
                "ts": "1700000000000"
            }]
        });

        let ticker = parse_ticker(&msg).unwrap();

        // Missing fields should be None
        assert_eq!(ticker.bid_volume, None);
        assert_eq!(ticker.ask_volume, None);

        // But prices should still be parsed
        assert_eq!(ticker.bid, Some(Price::new(dec!(49999.00))));
        assert_eq!(ticker.ask, Some(Price::new(dec!(50001.00))));
    }
}
