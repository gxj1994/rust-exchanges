//! Gate.io market parser.
//!
//! Parses market data from Gate.io spot API.

use super::parse_decimal;
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::common::currency::MinMax,
    types::financial::{Amount, Price},
    types::{
        Market, Ohlcv, Symbol,
        market::{MarketLimits, MarketPrecision, MarketType},
    },
};
use rust_decimal::Decimal;
use serde_json::Value;

/// Parse a single currency pair from Gate.io spot API.
///
/// # Gate.io Spot Market Response
///
/// ```json
/// {
///     "id": "BTC_USDT",
///     "base": "BTC",
///     "quote": "USDT",
///     "fee": "0.2",
///     "min_base_amount": "0.0001",
///     "min_quote_amount": "1",
///     "amount_precision": 8,
///     "precision": 8,
///     "trade_status": "tradable"
/// }
/// ```
pub fn parse_currency_pair(data: &Value) -> Result<Market> {
    let id = data["id"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("id")))?
        .to_string();

    let base = data["base"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("base")))?
        .to_string();

    let quote = data["quote"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("quote")))?
        .to_string();

    // Construct unified symbol: BTC/USDT
    let symbol_str = format!("{}/{}", base, quote);

    let fee = parse_decimal(data, "fee").unwrap_or_default();

    let min_base_amount = parse_decimal(data, "min_base_amount");
    let min_quote_amount = parse_decimal(data, "min_quote_amount");

    let amount_precision = data["amount_precision"].as_u64().unwrap_or(8) as u8;
    let price_precision = data["precision"].as_u64().unwrap_or(8) as u8;

    // Calculate tick size from precision: 10^(-precision)
    let price_tick = Decimal::from_f64_retain(10_f64.powi(-(price_precision as i32)))
        .unwrap_or(Decimal::new(1, price_precision as u32));
    let amount_step = Decimal::from_f64_retain(10_f64.powi(-(amount_precision as i32)))
        .unwrap_or(Decimal::new(1, amount_precision as u32));

    let trade_status = data["trade_status"].as_str().unwrap_or("");
    let active = trade_status == "tradable";

    Ok(Market {
        id,
        symbol: Symbol::new_unchecked(&symbol_str),
        parsed_symbol: None,
        base,
        quote,
        settle: None,
        base_id: None,
        quote_id: None,
        settle_id: None,
        market_type: MarketType::Spot,
        active,
        margin: false,
        contract: Some(false),
        linear: None,
        inverse: None,
        contract_size: None,
        expiry: None,
        expiry_datetime: None,
        strike: None,
        option_type: None,
        precision: MarketPrecision {
            price: Some(price_tick),
            amount: Some(amount_step),
            base: Some(amount_precision as u32),
            quote: Some(price_precision as u32),
        },
        limits: MarketLimits {
            amount: Some(MinMax {
                min: min_base_amount,
                max: None,
            }),
            price: None,
            cost: Some(MinMax {
                min: min_quote_amount,
                max: None,
            }),
            leverage: None,
        },
        maker: Some(fee),
        taker: Some(fee),
        percentage: Some(false),
        tier_based: Some(false),
        fee_side: None,
        info: Default::default(),
    })
}

/// Parse OHLCV (candlestick) data from Gate.io API.
///
/// # Gate.io Candlestick Response Format
///
/// Gate.io returns candlesticks as an array of arrays:
/// ```json
/// [
///   ["1609917600", "100.5", "40050", "40100", "39900", "40000", "20.5"],
///   ...
/// ]
/// ```
///
/// Fields: `[timestamp, quote_volume, close, high, low, open, base_volume, is_complete]`
///
/// # Arguments
///
/// * `data` - Single candlestick array
/// * `market` - Optional market information
///
/// # Returns
///
/// Returns a CCXT Ohlcv structure.
pub fn parse_ohlcv(data: &Value, _market: Option<&Market>) -> Result<Ohlcv> {
    let candle = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_format("candle", "Expected array")))?;

    if candle.len() < 7 {
        return Err(Error::from(ParseError::invalid_format(
            "candle",
            "Expected [timestamp, quote_volume, close, high, low, open, base_volume, is_complete]",
        )));
    }

    // Parse timestamp (in seconds, convert to milliseconds)
    let timestamp = if let Some(ts) = candle[0].as_str() {
        ts.parse::<i64>()
            .map_err(|e| Error::from(ParseError::invalid_format("timestamp", e.to_string())))?
            * 1000
    } else if let Some(ts) = candle[0].as_i64() {
        ts * 1000
    } else {
        return Err(Error::from(ParseError::missing_field("timestamp")));
    };

    // Gate.io candle format: [timestamp, quote_volume, close, high, low, open, base_volume, is_complete]
    let open = Price::new(parse_decimal_value(&candle[5])?);
    let high = Price::new(parse_decimal_value(&candle[3])?);
    let low = Price::new(parse_decimal_value(&candle[4])?);
    let close = Price::new(parse_decimal_value(&candle[2])?);
    let volume = Amount::new(parse_decimal_value(&candle[6])?);

    Ok(Ohlcv {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    })
}

/// Parse WebSocket OHLCV (candlestick) data from Gate.io.
///
/// # WebSocket Candlestick Response
///
/// ## Spot Format (object)
/// ```json
/// {
///     "t": "1777141440",
///     "v": "1739.3032302",
///     "c": "77295.6",
///     "h": "77295.6",
///     "l": "77291.8",
///     "o": "77291.8",
///     "a": "0.022503",
///     "n": "1m_BTC_USDT"
/// }
/// ```
///
/// ## Futures Format (array with one object)
/// ```json
/// [{"t":1777141689,"c":"77272","h":"77272","l":"77272","o":"77272","a":"61.8176","n":"1s_BTC_USDT","w":true,"v":8}]
/// ```
///
/// Note: Different from REST API which uses array format.
pub fn parse_ws_ohlcv(data: &Value) -> Result<Ohlcv> {
    // Handle both spot (object) and futures (array) formats
    let candle_data = if data.is_array() {
        // Futures: array with one candle object
        data.as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::from(ParseError::invalid_format("candle", "Empty array")))?
    } else {
        // Spot: single object
        data
    };

    // Parse timestamp (in seconds, convert to milliseconds)
    let timestamp = if let Some(ts) = candle_data["t"].as_str() {
        ts.parse::<i64>()
            .map_err(|e| Error::from(ParseError::invalid_format("timestamp", e.to_string())))?
            * 1000
    } else if let Some(ts) = candle_data["t"].as_i64() {
        ts * 1000
    } else {
        return Err(Error::from(ParseError::missing_field("t")));
    };

    // Parse OHLCV fields from object format
    let open = Price::new(
        parse_decimal(candle_data, "o")
            .ok_or_else(|| Error::from(ParseError::missing_field("o")))?,
    );
    let high = Price::new(
        parse_decimal(candle_data, "h")
            .ok_or_else(|| Error::from(ParseError::missing_field("h")))?,
    );
    let low = Price::new(
        parse_decimal(candle_data, "l")
            .ok_or_else(|| Error::from(ParseError::missing_field("l")))?,
    );
    let close = Price::new(
        parse_decimal(candle_data, "c")
            .ok_or_else(|| Error::from(ParseError::missing_field("c")))?,
    );
    let volume = Amount::new(
        parse_decimal(candle_data, "a")
            .ok_or_else(|| Error::from(ParseError::missing_field("a")))?,
    );

    Ok(Ohlcv {
        timestamp,
        open,
        high,
        low,
        close,
        volume,
    })
}

/// Helper function to parse a decimal value from string or number.
fn parse_decimal_value(value: &Value) -> Result<Decimal> {
    if let Some(s) = value.as_str() {
        s.parse::<Decimal>()
            .map_err(|e| Error::from(ParseError::invalid_format("decimal", e.to_string())))
    } else if let Some(f) = value.as_f64() {
        Ok(Decimal::from_f64_retain(f).ok_or_else(|| {
            Error::from(ParseError::invalid_format("decimal", "Invalid float value"))
        })?)
    } else {
        Err(Error::from(ParseError::missing_field("value")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_currency_pair() {
        let data = serde_json::json!({
            "id": "BTC_USDT",
            "base": "BTC",
            "quote": "USDT",
            "fee": "0.2",
            "min_base_amount": "0.0001",
            "min_quote_amount": "1",
            "amount_precision": 8,
            "precision": 8,
            "trade_status": "tradable"
        });

        let market = parse_currency_pair(&data).unwrap();
        assert_eq!(market.id, "BTC_USDT");
        assert_eq!(market.symbol.to_string(), "BTC/USDT");
        assert_eq!(market.base, "BTC");
        assert_eq!(market.quote, "USDT");
        assert!(market.active);
    }

    #[test]
    fn test_parse_inactive_market() {
        let data = serde_json::json!({
            "id": "DEL_USDT",
            "base": "DEL",
            "quote": "USDT",
            "fee": "0.2",
            "min_base_amount": "1",
            "min_quote_amount": "0.1",
            "amount_precision": 4,
            "precision": 6,
            "trade_status": "delisting"
        });

        let market = parse_currency_pair(&data).unwrap();
        assert!(!market.active);
    }
}

// ============================================================================
// Contract Market Parsing
// ============================================================================

/// Parse a contract market from Gate.io futures API.
///
/// # Gate.io Contract Market Response
///
/// ```json
/// {
///     "name": "BTC_USDT",
///     "underlying": "BTC_USDT",
///     "description": "BTC_USDT指数",
///     "quanto_multiplier": "0.0001",
///     "ref_discount_rate": "0.9",
///     "order_price_deviate": "0.5",
///     "delivery_date": 1610611200,
///     "mark_type": "index",
///     "mark_price": "42000.5",
///     "index_price": "42000.0",
///     "funding_rate_indicative": "0.0001",
///     "funding_rate": "0.0001",
///     "funding_next_apply": 1610611200,
///     "short_users": 100,
///     "long_users": 100,
///     "price_change_rate": "0.01",
///     "trade_size": 1000000,
///     "position_size": 500000,
///     "open_interest": "1000000",
///     "open_interest_value": "42000000000",
///     "limit_price": "50000",
///     "maker_fee_rate": "-0.00025",
///     "taker_fee_rate": "0.00075",
///     "order_price_round": "0.1",
///     "order_size_min": 1,
///     "order_size_max": 1000000,
///     "leverage_min": 1,
///     "leverage_max": 100,
///     "risk_limit_max": "1000000",
///     "risk_limit_step": "500000",
///     "risk_limit_base": "1000000",
///     "in_delisting": false,
///     "orders_limit": 50,
///     "enable_bonus": true,
///     "enable_credit": true,
///     "create_time": 1610611200,
///     "funding_interval": 28800
/// }
/// ```
pub fn parse_contract_market(data: &Value, settle: &str) -> Result<Market> {
    let name = data["name"]
        .as_str()
        .ok_or_else(|| Error::from(ParseError::missing_field("name")))?
        .to_string();

    // Try to get underlying field first, fallback to parsing from name
    // Testnet may not have underlying field
    let underlying = data["underlying"].as_str().unwrap_or("").to_string();

    // Parse base and quote from underlying or name (e.g., "BTC_USDT" -> base="BTC", quote="USDT")
    let (base, quote) = if !underlying.is_empty() && underlying.contains('_') {
        let parts: Vec<&str> = underlying.splitn(2, '_').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (underlying.clone(), settle.to_uppercase())
        }
    } else if name.contains('_') {
        // Fallback: parse from name field (for testnet)
        let parts: Vec<&str> = name.splitn(2, '_').collect();
        if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (name.clone(), settle.to_uppercase())
        }
    } else {
        (name.clone(), settle.to_uppercase())
    };

    // Construct unified symbol: BTC/USDT:USDT (for swap)
    let symbol_str = format!("{}/{}:{}", base, quote, settle.to_uppercase());

    let quanto_multiplier = parse_decimal(data, "quanto_multiplier").unwrap_or(Decimal::ONE);
    let maker_fee = parse_decimal(data, "maker_fee_rate").unwrap_or_default();
    let taker_fee = parse_decimal(data, "taker_fee_rate").unwrap_or_default();

    let in_delisting = data["in_delisting"].as_bool().unwrap_or(false);
    let active = !in_delisting;

    // Determine if linear or inverse
    let linear = if settle.to_lowercase() == quote.to_lowercase() {
        Some(true)
    } else if settle.to_lowercase() == base.to_lowercase() {
        Some(false)
    } else {
        None
    };

    let inverse = linear.map(|l| !l);

    Ok(Market {
        id: name,
        symbol: Symbol::new_unchecked(&symbol_str),
        parsed_symbol: None,
        base,
        quote,
        settle: Some(settle.to_uppercase()),
        base_id: None,
        quote_id: None,
        settle_id: Some(settle.to_string()),
        market_type: MarketType::Swap,
        active,
        margin: false,
        contract: Some(true),
        linear,
        inverse,
        contract_size: Some(quanto_multiplier),
        expiry: None,
        expiry_datetime: None,
        strike: None,
        option_type: None,
        precision: MarketPrecision::default(),
        limits: MarketLimits::default(),
        maker: Some(maker_fee),
        taker: Some(taker_fee),
        percentage: Some(false),
        tier_based: Some(false),
        fee_side: None,
        info: Default::default(),
    })
}
