//! Gate.io trade parser.
//!
//! Parses trade data from Gate.io spot and contract API responses.
//!
//! # API Response Format
//!
//! Gate.io trade response format:
//! ```json
//! {
//!   "id": 1234567890,
//!   "create_time": 1609917600,
//!   "create_time_ms": 1609917600000,
//!   "currency_pair": "BTC_USDT",
//!   "side": "buy",
//!   "role": "taker",
//!   "amount": "0.01",
//!   "price": "40000",
//!   "order_id": "9876543210",
//!   "fee": "0.00004",
//!   "fee_currency": "USDT"
//! }
//! ```

use super::parse_decimal;
use ccxt_core::{
    Result,
    error::{Error, ParseError},
    types::financial::{Amount, Cost, Price},
    types::{Market, OrderSide, Symbol, TakerOrMaker, Trade},
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::HashMap;

/// Convert JSON Value to HashMap<String, Value> for the `info` field.
fn value_to_info(value: &Value) -> HashMap<String, Value> {
    let mut map = HashMap::new();

    if let Some(obj) = value.as_object() {
        for (key, val) in obj {
            map.insert(key.clone(), val.clone());
        }
    }

    map
}

/// Parse trade from Gate.io spot API response.
///
/// # Arguments
///
/// * `data` - Gate.io trade response JSON
/// * `market` - Optional market information for symbol resolution
///
/// # Returns
///
/// Returns a CCXT Trade structure.
pub fn parse_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    // Parse symbol
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else {
        let currency_pair = data["currency_pair"]
            .as_str()
            .ok_or_else(|| Error::from(ParseError::missing_field("currency_pair")))?;

        // Convert BTC_USDT -> BTC/USDT
        let symbol_str = currency_pair.replace('_', "/");
        Symbol::new_unchecked(&symbol_str)
    };

    // Parse trade ID
    let id = data["id"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| data["id"].as_i64().map(|v| v.to_string()));

    // Parse timestamp
    // Gate API returns create_time_ms as a float string (e.g. "1777073829385.519000")
    let timestamp = data["create_time_ms"]
        .as_str()
        .and_then(|s| s.split('.').next())
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| data["create_time_ms"].as_i64())
        .or_else(|| {
            data["create_time"]
                .as_str()
                .and_then(|s| s.parse::<i64>().ok())
                .map(|t| t * 1000)
        })
        .or_else(|| data["create_time"].as_i64().map(|t| t * 1000))
        .unwrap_or(0);

    // Parse side
    let side = if let Some(side_str) = data["side"].as_str() {
        match side_str {
            "buy" => OrderSide::Buy,
            "sell" => OrderSide::Sell,
            _ => OrderSide::Buy, // Default to buy
        }
    } else {
        OrderSide::Buy
    };

    // Parse taker/maker role
    let taker_or_maker = if let Some(role_str) = data["role"].as_str() {
        match role_str {
            "taker" => Some(TakerOrMaker::Taker),
            "maker" => Some(TakerOrMaker::Maker),
            _ => None,
        }
    } else {
        None
    };

    // Parse price, amount, and cost
    let price = parse_decimal(data, "price");
    let amount = parse_decimal(data, "amount");

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    // Parse fee
    let fee_currency = data["fee_currency"].as_str().map(ToString::to_string);
    let fee_amount = parse_decimal(data, "fee");

    let fee = match (&fee_currency, &fee_amount) {
        (Some(currency), Some(amount)) => Some(ccxt_core::types::Fee {
            currency: currency.clone(),
            cost: *amount,
            rate: None,
        }),
        _ => None,
    };

    // Format datetime
    let datetime = if timestamp > 0 {
        Some(
            chrono::DateTime::from_timestamp(timestamp / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        )
    } else {
        None
    };

    Ok(Trade {
        id,
        order: data["order_id"]
            .as_str()
            .map(ToString::to_string)
            .or_else(|| data["order_id"].as_i64().map(|v| v.to_string())),
        timestamp,
        datetime,
        symbol,
        trade_type: None,
        side,
        taker_or_maker,
        price: Price::new(price.unwrap_or(Decimal::ZERO)),
        amount: Amount::new(amount.unwrap_or(Decimal::ZERO)),
        cost: cost.map(Cost::new),
        fee,
        info: value_to_info(data),
    })
}

/// Parse multiple trades from Gate.io API response.
///
/// # Arguments
///
/// * `data` - Array of trade objects
/// * `market` - Optional market information
///
/// # Returns
///
/// Returns a vector of parsed trades.
pub fn parse_trades(data: &Value, market: Option<&Market>) -> Result<Vec<Trade>> {
    let trades_array = data
        .as_array()
        .ok_or_else(|| Error::from(ParseError::invalid_format("data", "Expected array")))?;

    trades_array
        .iter()
        .map(|trade_data| parse_trade(trade_data, market))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_trade_basic() {
        let data = json!({
            "id": 1234567890i64,
            "create_time_ms": 1609917600000i64,
            "currency_pair": "BTC_USDT",
            "side": "buy",
            "role": "taker",
            "amount": "0.01",
            "price": "40000",
            "order_id": "9876543210",
            "fee": "0.00004",
            "fee_currency": "USDT"
        });

        let trade = parse_trade(&data, None).unwrap();

        assert_eq!(trade.id, Some("1234567890".to_string()));
        assert_eq!(trade.order, Some("9876543210".to_string()));
        assert_eq!(trade.side, OrderSide::Buy);
        assert_eq!(trade.taker_or_maker, Some(TakerOrMaker::Taker));
        assert_eq!(
            trade.price,
            Price::new(rust_decimal::Decimal::new(40000, 0))
        );
        assert_eq!(trade.amount, Amount::new(rust_decimal::Decimal::new(1, 2)));
    }

    #[test]
    fn test_parse_trade_sell() {
        let data = json!({
            "id": 9876543210i64,
            "create_time_ms": 1609917600000i64,
            "currency_pair": "ETH_USDT",
            "side": "sell",
            "role": "maker",
            "amount": "1.5",
            "price": "3000"
        });

        let trade = parse_trade(&data, None).unwrap();

        assert_eq!(trade.side, OrderSide::Sell);
        assert_eq!(trade.taker_or_maker, Some(TakerOrMaker::Maker));
        assert_eq!(trade.amount, Amount::new(rust_decimal::Decimal::new(15, 1)));
    }

    #[test]
    fn test_parse_trade_with_seconds_timestamp() {
        let data = json!({
            "id": 5555555555i64,
            "create_time": 1609917600i64,
            "currency_pair": "BTC_USDT",
            "side": "buy",
            "amount": "0.1",
            "price": "50000"
        });

        let trade = parse_trade(&data, None).unwrap();

        // Should convert seconds to milliseconds
        assert_eq!(trade.timestamp, 1609917600000);
    }
}

/// Parse WebSocket trade update from Gate.io.
///
/// # WebSocket Trade Response (Spot)
///
/// ```json
/// {
///     "id": 1234567890,
///     "create_time": 1609917600,
///     "create_time_ms": 1609917600000,
///     "currency_pair": "BTC_USDT",
///     "side": "buy",
///     "role": "taker",
///     "amount": "0.01",
///     "price": "40000"
/// }
/// ```
///
/// # WebSocket Trade Response (Futures)
///
/// ```json
/// {
///     "id": 1234567890,
///     "create_time": 1609917600,
///     "contract": "BTC_USDT",
///     "size": 1,
///     "price": "40000",
///     "text": "order123"
/// }
/// ```
pub fn parse_ws_trade(data: &Value, market: Option<&Market>) -> Result<Trade> {
    // Detect if it's a futures trade (has "contract" field) or spot trade (has "currency_pair")
    let symbol: Symbol = if let Some(m) = market {
        m.symbol.clone()
    } else if let Some(currency_pair) = data["currency_pair"].as_str() {
        // Spot: BTC_USDT -> BTC/USDT
        let symbol_str = currency_pair.replace('_', "/");
        Symbol::new_unchecked(&symbol_str)
    } else if let Some(contract) = data["contract"].as_str() {
        // Futures: BTC_USDT -> BTC/USDT:USDT
        let base_quote = contract.replace('_', "/");
        let unified = format!("{}:USDT", base_quote);
        Symbol::new_unchecked(&unified)
    } else {
        return Err(Error::from(ParseError::missing_field(
            "currency_pair or contract",
        )));
    };

    // Parse trade ID
    let id = data["id"]
        .as_str()
        .map(ToString::to_string)
        .or_else(|| data["id"].as_i64().map(|v| v.to_string()));

    // Parse timestamp
    let timestamp = data["create_time_ms"]
        .as_str()
        .and_then(|s| s.split('.').next())
        .and_then(|s| s.parse::<i64>().ok())
        .or_else(|| data["create_time_ms"].as_i64())
        .or_else(|| data["create_time"].as_i64().map(|t| t * 1000))
        .unwrap_or(0);

    // Parse side
    let side = if let Some(side_str) = data["side"].as_str() {
        match side_str {
            "buy" => OrderSide::Buy,
            "sell" => OrderSide::Sell,
            _ => OrderSide::Buy,
        }
    } else {
        OrderSide::Buy
    };

    // Parse taker/maker role
    let taker_or_maker = if let Some(role_str) = data["role"].as_str() {
        match role_str {
            "taker" => Some(TakerOrMaker::Taker),
            "maker" => Some(TakerOrMaker::Maker),
            _ => None,
        }
    } else {
        None
    };

    // Parse price, amount, and cost
    // Note: Futures uses "size" instead of "amount"
    let price = parse_decimal(data, "price");
    let amount = parse_decimal(data, "amount").or_else(|| parse_decimal(data, "size"));

    let cost = match (price, amount) {
        (Some(p), Some(a)) => Some(p * a),
        _ => None,
    };

    // Parse fee
    let fee_currency = data["fee_currency"].as_str().map(ToString::to_string);
    let fee_amount = parse_decimal(data, "fee");

    let fee = match (&fee_currency, &fee_amount) {
        (Some(currency), Some(amount)) => Some(ccxt_core::types::Fee {
            currency: currency.clone(),
            cost: *amount,
            rate: None,
        }),
        _ => None,
    };

    // Format datetime
    let datetime = if timestamp > 0 {
        Some(
            chrono::DateTime::from_timestamp(timestamp / 1000, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default(),
        )
    } else {
        None
    };

    Ok(Trade {
        id,
        order: data["order_id"]
            .as_str()
            .map(ToString::to_string)
            .or_else(|| data["order_id"].as_i64().map(|v| v.to_string()))
            .or_else(|| data["text"].as_str().map(ToString::to_string)),
        timestamp,
        datetime,
        symbol,
        trade_type: None,
        side,
        taker_or_maker,
        price: Price::new(price.unwrap_or(Decimal::ZERO)),
        amount: Amount::new(amount.unwrap_or(Decimal::ZERO)),
        cost: cost.map(Cost::new),
        fee,
        info: value_to_info(data),
    })
}
