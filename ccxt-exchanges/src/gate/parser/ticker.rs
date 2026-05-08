//! Gate.io ticker parser.

use super::{parse_decimal, parse_f64};
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::{
        Market, Symbol, Ticker,
        financial::{Amount, Price},
    },
};
use serde_json::Value;

/// Parse ticker from Gate.io spot API response.
///
/// # Gate.io Ticker Response
///
/// ```json
/// {
///     "currency_pair": "BTC_USDT",
///     "last": "50000.0",
///     "lowest_ask": "50001.0",
///     "highest_bid": "49999.0",
///     "change_percentage": "2.5",
///     "base_volume": "1000.5",
///     "quote_volume": "50000000.0",
///     "high_24h": "51000.0",
///     "low_24h": "49000.0"
/// }
/// ```
pub fn parse_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        let currency_pair = data["currency_pair"]
            .as_str()
            .or_else(|| data["symbol"].as_str())
            .ok_or_else(|| Error::from(ParseError::missing_field("currency_pair")))?;

        // Convert BTC_USDT -> BTC/USDT
        let unified = currency_pair.replace('_', "/");
        Symbol::new_unchecked(&unified)
    };

    let last = parse_decimal(data, "last");
    let timestamp = parse_f64(data, "timestamp").map(|t| t as i64);

    Ok(Ticker {
        symbol,
        timestamp: timestamp.unwrap_or(0),
        datetime: timestamp.map(|t| {
            chrono::DateTime::from_timestamp_millis(t)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        high: parse_decimal(data, "high_24h").map(Price::new),
        low: parse_decimal(data, "low_24h").map(Price::new),
        bid: parse_decimal(data, "highest_bid").map(Price::new),
        bid_volume: parse_decimal(data, "bid_size").map(Amount::new),
        ask: parse_decimal(data, "lowest_ask").map(Price::new),
        ask_volume: parse_decimal(data, "ask_size").map(Amount::new),
        vwap: None,
        open: None,
        close: last.map(Price::new),
        last: last.map(Price::new),
        previous_close: parse_decimal(data, "previous_close").map(Price::new),
        change: parse_decimal(data, "change").map(Price::new),
        percentage: parse_decimal(data, "change_percentage"),
        average: None,
        base_volume: parse_decimal(data, "base_volume").map(Amount::new),
        quote_volume: parse_decimal(data, "quote_volume").map(Amount::new),
        funding_rate: None,
        open_interest: None,
        index_price: None,
        mark_price: None,
        info: Default::default(),
    })
}

/// Parse contract ticker from Gate.io futures API.
///
/// # Gate.io Contract Ticker Response
///
/// ```json
/// {
///     "contract": "BTC_USDT",
///     "last": "42000.5",
///     "change_percentage": "1.5",
///     "funding_rate": "0.0001",
///     "funding_rate_indicative": "0.0001",
///     "mark_price": "42000.0",
///     "index_price": "42000.0",
///     "total_size": "1000000",
///     "volume_24h": "50000000",
///     "volume_24h_base": "1200",
///     "volume_24h_quote": "50400000",
///     "high_24h": "43000",
///     "low_24h": "41000"
/// }
/// ```
pub fn parse_contract_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        let contract = data["contract"]
            .as_str()
            .or_else(|| data["name"].as_str())
            .unwrap_or("UNKNOWN");

        // Convert BTC_USDT -> BTC/USDT:USDT (futures format)
        let base_quote = contract.replace('_', "/");
        let unified = format!("{}:USDT", base_quote);
        Symbol::new_unchecked(&unified)
    };

    let last = parse_decimal(data, "last");
    let timestamp = parse_f64(data, "timestamp").map(|t| t as i64);

    // Gate contract uses different field names for bid/ask
    let bid = parse_decimal(data, "highest_bid");
    let ask = parse_decimal(data, "lowest_ask");
    let bid_volume = parse_decimal(data, "highest_size");
    let ask_volume = parse_decimal(data, "lowest_size");

    Ok(Ticker {
        symbol,
        timestamp: timestamp.unwrap_or(0),
        datetime: timestamp.map(|t| {
            chrono::DateTime::from_timestamp_millis(t)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }),
        high: parse_decimal(data, "high_24h").map(Price::new),
        low: parse_decimal(data, "low_24h").map(Price::new),
        bid: bid.map(Price::new),
        bid_volume: bid_volume.map(Amount::new),
        ask: ask.map(Price::new),
        ask_volume: ask_volume.map(Amount::new),
        vwap: None,
        open: None,
        close: last.map(Price::new),
        last: last.map(Price::new),
        previous_close: None,
        change: None,
        percentage: parse_decimal(data, "change_percentage"),
        average: None,
        base_volume: parse_decimal(data, "volume_24h_base").map(Amount::new),
        quote_volume: parse_decimal(data, "volume_24h_quote").map(Amount::new),
        funding_rate: parse_decimal(data, "funding_rate"),
        open_interest: parse_decimal(data, "total_size"),
        index_price: parse_decimal(data, "index_price").map(Price::new),
        mark_price: parse_decimal(data, "mark_price").map(Price::new),
        info: data
            .as_object()
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default(),
    })
}

/// Parse WebSocket ticker update from Gate.io.
///
/// # WebSocket Ticker Response (Spot)
///
/// ```json
/// {
///     "currency_pair": "BTC_USDT",
///     "last": "50000.0",
///     "timestamp": 1234567890
/// }
/// ```
///
/// # WebSocket Ticker Response (Futures)
///
/// ```json
/// {
///     "contract": "BTC_USDT",
///     "last": "77327.0",
///     "change_percentage": "-0.4232",
///     "total_size": "606355066",
///     "volume_24h": "232157321",
///     "volume_24h_base": "23215.7321",
///     "volume_24h_quote": "1795202922",
///     "mark_price": "77329.9",
///     "funding_rate": "-0.000019",
///     "index_price": "77370.7",
///     "low_24h": "77088.0",
///     "high_24h": "77838.6"
/// }
/// ```
pub fn parse_ws_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    // Detect if it's a futures ticker (has "contract" field) or spot ticker (has "currency_pair")
    if data.get("contract").is_some() {
        // Futures/swap ticker
        parse_contract_ticker(data, market)
    } else {
        // Spot ticker
        parse_ticker(data, market)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ticker() {
        let data = serde_json::json!({
            "currency_pair": "BTC_USDT",
            "last": "50000.0",
            "lowest_ask": "50001.0",
            "highest_bid": "49999.0",
            "change_percentage": "2.5",
            "base_volume": "1000.5",
            "quote_volume": "50000000.0",
            "high_24h": "51000.0",
            "low_24h": "49000.0"
        });

        let ticker = parse_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol.to_string(), "BTC/USDT");
        assert_eq!(ticker.last.unwrap().to_string(), "50000.0");
        assert_eq!(ticker.bid.unwrap().to_string(), "49999.0");
        assert_eq!(ticker.ask.unwrap().to_string(), "50001.0");
        assert_eq!(ticker.base_volume.unwrap().to_string(), "1000.5");
    }
}
