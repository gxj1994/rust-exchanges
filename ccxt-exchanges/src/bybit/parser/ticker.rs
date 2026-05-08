//! Ticker parser for Bybit.

use ccxt_core::{
    Result,
    parser_utils::{parse_decimal, parse_timestamp, value_to_hashmap},
    types::financial::{Amount, Price},
    types::{Market, MarketType, Symbol, Ticker},
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse ticker data from Bybit ticker response.
///
/// # Arguments
///
/// * `data` - Bybit ticker data JSON object
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
///
/// # Market Type Differences
///
/// Bybit V5 API returns different fields based on market type:
/// - **Linear/Inverse**: Uses `bid1Price`, `ask1Price`, `bid1Size`, `ask1Size`
/// - **Option**: Uses `bidPrice`, `askPrice`, `bidSize`, `askSize`
/// - **Spot**: Does NOT include bid/ask fields at all
///
/// # Timestamp Note
///
/// Bybit REST API ticker data does NOT include timestamp in the ticker object.
/// The timestamp is in the outer response: `{"retCode":0,"result":{...},"time":1234567890}`
/// Callers should extract `time` from response and pass it to this function.
/// If `external_timestamp` is None, falls back to `time`/`timestamp` fields in data,
/// or current time as last resort.
pub fn parse_ticker(data: &Value, market: Option<&Market>) -> Result<Ticker> {
    let market_type = market.map(|m| m.market_type).unwrap_or(MarketType::Spot);
    parse_ticker_internal(data, market, None, market_type)
}

/// Parse ticker data from REST API response with external timestamp.
///
/// # Arguments
///
/// * `data` - Bybit ticker data JSON object (from `result.list[]`)
/// * `market` - Optional market information for symbol resolution
/// * `response_time` - Timestamp from outer response (`time` field)
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
pub fn parse_ticker_with_time(
    data: &Value,
    market: Option<&Market>,
    response_time: Option<i64>,
) -> Result<Ticker> {
    let market_type = market.map(|m| m.market_type).unwrap_or(MarketType::Spot);
    parse_ticker_internal(data, market, response_time, market_type)
}

/// Parse ticker data from WebSocket message with external timestamp.
///
/// # Arguments
///
/// * `data` - Bybit ticker data JSON object (from WebSocket `data` field)
/// * `market` - Optional market information for symbol resolution
/// * `ws_timestamp` - Optional timestamp from WebSocket outer message (`ts` field)
///
/// # Returns
///
/// Returns a CCXT [`Ticker`] structure.
///
/// # Note
///
/// WebSocket ticker messages have the timestamp in the outer message,
/// not in the `data` object. This function accepts the timestamp separately.
pub fn parse_ticker_with_ws_timestamp(
    data: &Value,
    market: Option<&Market>,
    ws_timestamp: Option<i64>,
) -> Result<Ticker> {
    // Infer market type from market info or default to Spot
    let market_type = market.map(|m| m.market_type).unwrap_or(MarketType::Spot);
    parse_ticker_internal(data, market, ws_timestamp, market_type)
}

/// Internal ticker parsing implementation.
///
/// Handles market-type-specific field differences:
/// - Linear/Inverse: `bid1Price`, `ask1Price`, `bid1Size`, `ask1Size`
/// - Option: `bidPrice`, `askPrice`, `bidSize`, `askSize`
/// - Spot: No bid/ask fields (returns None)
fn parse_ticker_internal(
    data: &Value,
    market: Option<&Market>,
    external_timestamp: Option<i64>,
    market_type: MarketType,
) -> Result<Ticker> {
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        // Try to construct symbol from symbol field
        // Bybit uses concatenated format like "BTCUSDT"
        data["symbol"]
            .as_str()
            .map(|s| Symbol::new_unchecked(s))
            .ok_or_else(|| {
                ccxt_core::Error::from(ccxt_core::ParseError::missing_field("symbol"))
                    .context("Failed to parse ticker: missing symbol identifier")
            })?
    };

    // Timestamp priority:
    // 1. External timestamp from response outer layer (REST: `time`, WebSocket: `ts`)
    // 2. `time` or `timestamp` fields in data (for backwards compatibility)
    // 3. Current time as last resort fallback
    let timestamp = external_timestamp.unwrap_or_else(|| {
        parse_timestamp(data, "time")
            .or_else(|| parse_timestamp(data, "timestamp"))
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis())
    });

    // Parse bid/ask fields based on market type
    // Linear/Inverse: bid1Price, ask1Price, bid1Size, ask1Size
    // Option: bidPrice, askPrice, bidSize, askSize
    // Spot: No bid/ask fields
    let (bid, bid_volume, ask, ask_volume) = match market_type {
        MarketType::Option => {
            // Option uses different field names
            (
                parse_decimal(data, "bidPrice").map(Price::new),
                parse_decimal(data, "bidSize").map(Amount::new),
                parse_decimal(data, "askPrice").map(Price::new),
                parse_decimal(data, "askSize").map(Amount::new),
            )
        }
        MarketType::Spot => {
            // Spot ticker does NOT include bid/ask fields per Bybit V5 API
            // https://bybit-exchange.github.io/docs/v5/websocket/public/ticker
            (None, None, None, None)
        }
        _ => {
            // Linear/Inverse (Swap/Futures) uses bid1Price, ask1Price, etc.
            (
                parse_decimal(data, "bid1Price").map(Price::new),
                parse_decimal(data, "bid1Size").map(Amount::new),
                parse_decimal(data, "ask1Price").map(Price::new),
                parse_decimal(data, "ask1Size").map(Amount::new),
            )
        }
    };

    // Parse USD index price for Spot markets
    let usd_index_price = parse_decimal(data, "usdIndexPrice").map(Price::new);

    // Use usdIndexPrice as index_price for Spot if available
    let index_price = match market_type {
        MarketType::Spot => usd_index_price,
        _ => parse_decimal(data, "indexPrice").map(Price::new),
    };

    Ok(Ticker {
        symbol,
        timestamp,
        datetime: ccxt_core::parser_utils::timestamp_to_datetime(timestamp),
        high: parse_decimal(data, "highPrice24h").map(Price::new),
        low: parse_decimal(data, "lowPrice24h").map(Price::new),
        bid,
        bid_volume,
        ask,
        ask_volume,
        vwap: None,
        open: parse_decimal(data, "prevPrice24h").map(Price::new),
        close: parse_decimal(data, "lastPrice").map(Price::new),
        last: parse_decimal(data, "lastPrice").map(Price::new),
        previous_close: parse_decimal(data, "prevPrice24h").map(Price::new),
        change: parse_decimal(data, "price24hPcnt")
            .and_then(|pct| parse_decimal(data, "prevPrice24h").map(|prev| Price::new(prev * pct))),
        percentage: parse_decimal(data, "price24hPcnt").map(|p| p * Decimal::from(100)),
        average: None,
        base_volume: parse_decimal(data, "volume24h").map(Amount::new),
        quote_volume: parse_decimal(data, "turnover24h").map(Amount::new),
        funding_rate: parse_decimal(data, "fundingRate"),
        open_interest: parse_decimal(data, "openInterest"),
        index_price,
        mark_price: parse_decimal(data, "markPrice").map(Price::new),
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
            "symbol": "BTCUSDT",
            "lastPrice": "50000.00",
            "highPrice24h": "51000.00",
            "lowPrice24h": "49000.00",
            "bid1Price": "49999.00",
            "ask1Price": "50001.00",
            "volume24h": "1000.5"
            // Note: Bybit REST API ticker does NOT include timestamp field
        });

        let ticker = parse_ticker(&data, None).unwrap();
        assert_eq!(ticker.symbol, Symbol::new_unchecked("BTCUSDT"));
        assert_eq!(ticker.last, Some(Price::new(dec!(50000.00))));
        assert_eq!(ticker.high, Some(Price::new(dec!(51000.00))));
        assert_eq!(ticker.low, Some(Price::new(dec!(49000.00))));
        // REST API ticker uses current time as timestamp
        assert!(ticker.timestamp > 0);
    }
}
